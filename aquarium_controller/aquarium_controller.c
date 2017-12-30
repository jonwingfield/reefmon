#include <stdio.h>
#include <stdlib.h>
#include <avr/io.h>
#include <avr/interrupt.h>
#include <util/delay.h>
#include <stdbool.h>
#include <string.h>
#include "ds18b20.h"
#include "commands.h"
#include "dbg.h"
#include "SMBSlave.h"
#include "rht03.h"

#define BAUD 9600
#define MYUBRR (F_CPU/16/(BAUD-1))

#define sbi(var, mask)   ((var) |= (uint8_t)(1 << mask))
#define cbi(var, mask)   ((var) &= (uint8_t)~(1 << mask))

#define c_to_f(temp)   (9.0 / 5.0 * (double)(temp) + 32.0)

#define set_channel1(a)  OCR0A = (a)
#define set_channel2(a)  OCR0B = (a)
#define set_channel3(a)  OCR1A = (a)
#define set_channel4(a)  OCR1B = (a)
#define set_channel5(a)  OCR2A = (a)
#define set_channel6(a)  OCR2B = (a)
#ifdef MEGA328PB
#define set_channel7(a)  OCR3A = (a)
#define set_channel8(a)  OCR3B = (a)
#endif

//Define functions
//======================
void ioinit(void);      // initializes IO
static int uart_putchar(char c, FILE *stream);
static int uart_getchar(FILE* stream);
/* static bool uart_available(void); */
void publish_depth_reading(void);
void publish_temp(void);
void publish_air_temp_humidity(void);
void set_channel(uint8_t channel_id, uint8_t intensity);
void ProcessMessage(SMBData* data);
void ProcessReceiveByte(SMBData* data);
inline static void OutputTemp(SMBData* smb);
inline static void OutputAirTempHumidity(SMBData* smb);
inline static void OutputDepth(SMBData* smb);
inline static void SetChannels(SMBData* smb);
inline static void UndefinedCommand(SMBData *smb);
inline static void crc_buf(uint8_t* buf, uint8_t len);
inline static bool crc_verify_buf(uint8_t* buf, uint8_t len);

static FILE mystdout = FDEV_SETUP_STREAM(uart_putchar, NULL, _FDEV_SETUP_WRITE);
static FILE uart_input = FDEV_SETUP_STREAM(NULL, uart_getchar, _FDEV_SETUP_READ);

ISR(BADISR_vect)
{
    printf("badisr\n");
}

static temp_info last_temp;
static uint16_t depth_reading;
static temp_humidity_info air_temp_humidity;

int main (void)
{
    ioinit(); //Setup IO pins and defaults

    SMBusInit();
    SMBEnable();

    sei();

    printf("Starting up!...\n");
    
    for (uint8_t j=0; j<6; j++) {
      set_channel(j+1, (j+1)*30);
    }

    uint8_t i=0;
    while (1) {
       if (i%3 == 0) {
         publish_temp();
       }
       if (i%5 == 0) {
         publish_air_temp_humidity();
       }

       publish_depth_reading();

       _delay_ms(1000);
       i++;
    }

    return(0);
}

void publish_temp(void) {
  temp_info temp;
  if (!therm_read_temp(&temp)) {
    printf("Error reading temperature\n");
  } else {
    cli();
    last_temp = temp;
    sei();
    printf("%u.%u\n", temp.major, temp.minor);
  }
}

void publish_air_temp_humidity(void) {
  temp_humidity_info temp_humidity;
  if (!rht03_temp_and_humidity(&temp_humidity)) {
    printf("Error reading air temperature/humidity\n");
  } else {
    cli();
    air_temp_humidity = temp_humidity;
    sei();
    printf("Got air temp: %u and humidity: %u\n", temp_humidity.temp, temp_humidity.humidity);
  }
}

void publish_depth_reading(void) {
    static uint16_t depth_readings[8];
    static uint8_t depth_reading_count = 0;
    static bool first = true;

    uint16_t current = 0;
    uint16_t reading;
    uint16_t sum = 0;
    // read 20 times and average it. There doesn't seem to be a ton of sensor noise until it's in an aquarium
    for (uint8_t i=0; i<20; i++) {
      reading = 0;
      ADCSRA |= (1 << ADSC);

      // wait for conversion to complete
      while (ADCSRA & (1 << ADSC));
      reading = ADCL;
      reading |= ((ADCH & 0b11) << 8);

      sum += reading;
      _delay_ms(1);
    }
    sum /= 20;
    
    printf("Raw reading: %u\n", current);

    if (first) {
      first = false;
      depth_readings[1] = current;
      depth_readings[2] = current;
      depth_readings[3] = current;
      depth_readings[4] = current;
      depth_readings[5] = current;
      depth_readings[6] = current;
      depth_readings[7] = current;
    }
    depth_readings[depth_reading_count] = current;
    depth_reading_count++;
    if (depth_reading_count > 7) {
      depth_reading_count = 0;
    }

    uint16_t average = 0;
    for (uint8_t i=0; i<8; i++) {
      average += depth_readings[i];
    }
    average /= 8;
    depth_reading = average;
    printf("Depth reading: %u\n", depth_reading);
}

void ProcessMessage(SMBData* smb) 
{
  if (smb->state == SMB_STATE_WRITE_REQUESTED) {
    switch (smb->rxBuffer[0]) // command code
    {
      case AQ_CMD_SETCHANNELS:
        SetChannels(smb);
        break;
      case AQ_CMD_GET_TEMP:
        OutputTemp(smb);
        break;
      case AQ_CMD_GET_AIR_TEMP_HUMIDITY:
        OutputAirTempHumidity(smb);
        break;
      case AQ_CMD_GET_DEPTH:
        OutputDepth(smb);
        break;
      default:
        UndefinedCommand(smb);
        break;
    }
  } else { 
    smb->state = SMB_STATE_IDLE;
  }
}

inline static void OutputTemp(SMBData* smb)
{
  smb->txBuffer[0] = (uint8_t)(last_temp.raw >> 8);
  smb->txBuffer[1] = (uint8_t)(last_temp.raw & 0xff);
  crc_buf(smb->txBuffer, 2);
  smb->txLength = 3;

  smb->state = SMB_STATE_READ_REQUESTED;
}

inline static void OutputAirTempHumidity(SMBData* smb)
{
  smb->txBuffer[0] = (uint8_t)(air_temp_humidity.temp >> 8);
  smb->txBuffer[1] = (uint8_t)(air_temp_humidity.temp & 0xff);
  smb->txBuffer[2] = (uint8_t)(air_temp_humidity.humidity >> 8);
  smb->txBuffer[3] = (uint8_t)(air_temp_humidity.humidity & 0xff);
  crc_buf(smb->txBuffer, 4);
  smb->txLength = 5;

  smb->state = SMB_STATE_READ_REQUESTED;
}

inline static void OutputDepth(SMBData* smb)
{
  smb->txBuffer[0] = (uint8_t)(depth_reading >> 8);
  smb->txBuffer[1] = (uint8_t)(depth_reading & 0xff);
  crc_buf(smb->txBuffer, 2);
  smb->txLength = 3;

  smb->state = SMB_STATE_READ_REQUESTED;
}
 
 
inline static void SetChannels(SMBData* smb) 
{
  /* uint8_t byteCount = smb->rxBuffer[1]; */
  if (crc_verify_buf(smb->rxBuffer+2, 8)) {
    set_channel1(smb->rxBuffer[2]);
    set_channel2(smb->rxBuffer[3]);
    set_channel3(smb->rxBuffer[4]);
    set_channel4(smb->rxBuffer[5]);
    set_channel5(smb->rxBuffer[6]);
    set_channel6(smb->rxBuffer[7]);
    set_channel7(smb->rxBuffer[8]);
    set_channel8(smb->rxBuffer[9]);

    smb->state = SMB_STATE_IDLE;
  } 
  else 
  {
    smb->error = TRUE;
    smb->state = SMB_STATE_IDLE;
  }
}

inline static void UndefinedCommand(SMBData *smb)
{
    smb->error = TRUE;
    smb->state = SMB_STATE_IDLE;
}

void set_all_channels(uint8_t intensity_i) {
  for (uint8_t j=0; j<6; j++) {
    set_channel(j+1, intensity_i);
  }
}

void set_channel(uint8_t channel_id, uint8_t intensity) {
  switch (channel_id) {
    case 1: set_channel1(intensity); break;
    case 2: set_channel2(intensity); break;
    case 3: set_channel3(intensity); break;
    case 4: set_channel4(intensity); break;
    case 5: set_channel5(intensity); break;
    case 6: set_channel6(intensity); break;
    case 7: set_channel7(intensity); break;
    case 8: set_channel8(intensity); break;
    default: break;
  }
}

#include <util/setbaud.h>
// P15 in the manual
void ioinit (void)
{
  // PWM Setup ////////////////////////
  // TIMER0
    DDRD |= (1 << DDD6);
    DDRD |= (1 << DDD5);

    // output enables
    TCCR0A |= (1 << COM0A1); // PD6
    TCCR0A |= (1 << COM0B1); // PD5

    // phase correct PWM mode
    TCCR0A |= (1 << WGM00);
    // IMPORTANT: Meanwell requires a PWM frequency of 100 - 1000hZ
    TCCR0B |= (1 << CS01) | (1 << CS00); // 64 prescaler
    // freq = fclk_io / (prescale * 510) = 245Hz

  // TIMER1 
    DDRB |= (1 << DDB1);
    DDRB |= (1 << DDB2);

    sbi(TCCR1A, COM1A1);
    sbi(TCCR1A, COM1B1);
    // 8-bit compare, phase corrected
    sbi(TCCR1A, WGM10);
    // IMPORTANT: Meanwell requires a PWM frequency of 100 - 1000hZ
    sbi(TCCR1B, CS10); // 64 prescaler
    sbi(TCCR1B, CS11); // 64 prescaler

  // TIMER2
    DDRD |= (1 << DDD3);
    DDRB |= (1 << DDB3);

    sbi(TCCR2A, COM2A1);
    sbi(TCCR2A, COM2B1);
    // phase corrected PWM
    sbi(TCCR2A, WGM20);
    // 256 prescaler
    sbi(TCCR2B, CS22);
    sbi(TCCR2B, CS21);

#ifdef MEGA328PB
    // TIMER3 (Atmega328pb only)
    DDRD |= (1 << DDD0);
    DDRD |= (1 << DDD2);

    sbi(TCCR3A, COM3A1);
    sbi(TCCR3A, COM3B1);
    // 8-bit compare, phase corrected
    sbi(TCCR3A, WGM30);
    // IMPORTANT: Meanwell requires a PWM frequency of 100 - 1000hZ
    sbi(TCCR3B, CS10); // 64 prescaler
    sbi(TCCR3B, CS11); // 64 prescaler
#endif

    // ADC Setup
    
    ADCSRA |= (1 << ADPS2) | (1 << ADPS1); // Prescaler = 64 since we're running at 8MHz. See p 250
    ADMUX |= (1 << REFS0); // use AVCC on pin. Make sure to have a .1uF cap on AVCC/gnd
    // ADMUX defaults to ADC0, nothing to set 
    ADCSRA |= (1 << ADEN); // enable ADC
    ADCSRA |= (1 << ADSC);  // free running mode, start conversion and let it run

    // UART Setup

    UBRR0H = UBRRH_VALUE;
    UBRR0L = UBRRL_VALUE;

#if USE_2X
    UCSR0A |= _BV(U2X0);
#else
    UCSR0A &= ~(_BV(U2X0));
#endif

    UCSR0C = _BV(UCSZ01) | _BV(UCSZ00); /* 8-bit data */
    UCSR0B = _BV(RXEN0) | _BV(TXEN0);   /* Enable RX and TX */

    stdout = &mystdout; //Required for printf init
    stdin = &uart_input;
}

static int uart_putchar(char c, FILE *stream)
{
    if (c == '\n') uart_putchar('\r', stream);
  
    loop_until_bit_is_set(UCSR0A, UDRE0);
    UDR0 = c;
    
    return 0;
}

static int uart_getchar(FILE* stream)
{
    loop_until_bit_is_set(UCSR0A, RXC0);
    return(UDR0);
}

/* static bool uart_available() { */
/*   return UCSR0A & RXC0; */
/* } */
//
//! Table of crc values stored in flas
const unsigned __flash char crcTable[256] =	{	
	0x00, 0x07, 0x0e, 0x09, 0x1c, 0x1b, 0x12, 0x15,
	0x38, 0x3f, 0x36, 0x31, 0x24, 0x23, 0x2a, 0x2d,
	0x70, 0x77, 0x7e, 0x79, 0x6c, 0x6b, 0x62, 0x65,
	0x48, 0x4f, 0x46, 0x41, 0x54, 0x53, 0x5a, 0x5d,
	0xe0, 0xe7, 0xee, 0xe9, 0xfc, 0xfb, 0xf2, 0xf5,
	0xd8, 0xdf, 0xd6, 0xd1, 0xc4, 0xc3, 0xca, 0xcd,
	0x90, 0x97, 0x9e, 0x99, 0x8c, 0x8b, 0x82, 0x85,
	0xa8, 0xaf, 0xa6, 0xa1, 0xb4, 0xb3, 0xba, 0xbd,
	0xc7, 0xc0, 0xc9, 0xce, 0xdb, 0xdc, 0xd5, 0xd2,
	0xff, 0xf8, 0xf1, 0xf6, 0xe3, 0xe4, 0xed, 0xea,
	0xb7, 0xb0, 0xb9, 0xbe, 0xab, 0xac, 0xa5, 0xa2,
	0x8f, 0x88, 0x81, 0x86, 0x93, 0x94, 0x9d, 0x9a,
	0x27, 0x20, 0x29, 0x2e, 0x3b, 0x3c, 0x35, 0x32,
	0x1f, 0x18, 0x11, 0x16, 0x03, 0x04, 0x0d, 0x0a,
	0x57, 0x50, 0x59, 0x5e, 0x4b, 0x4c, 0x45, 0x42,
	0x6f, 0x68, 0x61, 0x66, 0x73, 0x74, 0x7d, 0x7a,
	0x89, 0x8e, 0x87, 0x80, 0x95, 0x92, 0x9b, 0x9c,
	0xb1, 0xb6, 0xbf, 0xb8, 0xad, 0xaa, 0xa3, 0xa4,
	0xf9, 0xfe, 0xf7, 0xf0, 0xe5, 0xe2, 0xeb, 0xec,
	0xc1, 0xc6, 0xcf, 0xc8, 0xdd, 0xda, 0xd3, 0xd4,
	0x69, 0x6e, 0x67, 0x60, 0x75, 0x72, 0x7b, 0x7c,
	0x51, 0x56, 0x5f, 0x58, 0x4d, 0x4a, 0x43, 0x44,
	0x19, 0x1e, 0x17, 0x10, 0x05, 0x02, 0x0b, 0x0c,
	0x21, 0x26, 0x2f, 0x28, 0x3d, 0x3a, 0x33, 0x34,
	0x4e, 0x49, 0x40, 0x47, 0x52, 0x55, 0x5c, 0x5b,
	0x76, 0x71, 0x78, 0x7f, 0x6a, 0x6d, 0x64, 0x63,
	0x3e, 0x39, 0x30, 0x37, 0x22, 0x25, 0x2c, 0x2b,
	0x06, 0x01, 0x08, 0x0f, 0x1a, 0x1d, 0x14, 0x13,
	0xae, 0xa9, 0xa0, 0xa7, 0xb2, 0xb5, 0xbc, 0xbb,
	0x96, 0x91, 0x98, 0x9f, 0x8a, 0x8d, 0x84, 0x83,
	0xde, 0xd9, 0xd0, 0xd7, 0xc2, 0xc5, 0xcc, 0xcb,
	0xe6, 0xe1, 0xe8, 0xef, 0xfa, 0xfd, 0xf4, 0xf3
};


 /* \brief PEC CRC lookup function
  *
  * This function uses a table stored in flash to look up the
  * PEC value of one byte, using the PEC calculated so far as a
  * starting point.
  *
  */
inline static void crc_buf(uint8_t* buf, uint8_t len) 
{
  uint8_t crc = 0xff;

  for (uint8_t i=0; i<len; i++, buf++) {
    crc ^= *buf;
    crc = crcTable[crc];
  }

  *buf = crc;
}

inline static bool crc_verify_buf(uint8_t* buf, uint8_t len) 
{
  uint8_t crc = 0xff;

  for (uint8_t i=0; i<len; i++, buf++) {
    crc ^= *buf;
    crc = crcTable[crc];
  }

  return *buf == crc;
}

/* static void crcCalc(uint16_t crc, unsigned char data) */
/* { */
/*     unsigned char i;	// Counter for 8 shifts */

/*     crc ^= data;        // Initial XOR */

/*     i = 8; */
/*     do */
/*     { */
/*         if (crc & 0x80) */
/*         { */
/*             crc <<= 1; */
/*             crc ^= SMB_CRC_POLYNOME; */
/*         } */
/*         else */
/*         { */
/*             crc <<= 1; */
/*         } */
/*     } */
/*     while(--i); */

/*     return crc; */
/* } */
