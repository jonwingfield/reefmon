#include <avr/io.h>
#include <util/delay.h>
#include <avr/interrupt.h>
#include <util/crc16.h>
#include <stdbool.h>
#include "ds18b20.h"
#include "dbg.h"

#define THERM_INPUT_MODE()  THERM_DDR &= ~(1 << THERM_DD)
#define THERM_OUTPUT_MODE() THERM_DDR |= (1 << THERM_DD)
#define THERM_LOW()         THERM_PORT &= ~(1 << THERM_DQ)
#define THERM_HIGH()       THERM_PORT |= (1 << THERM_DQ)

// P15 in the manual
uint8_t therm_reset(void) 
{
    // send reset pulse 
    THERM_LOW();
    THERM_OUTPUT_MODE();
    _delay_us(480);

    // wait for presence pulse
    THERM_INPUT_MODE();
    _delay_us(60);

    // it's low if we're OK, high otherwise (pullup should keep it high)
    uint8_t i = (THERM_PIN & (1 << THERM_DQ));
    _delay_us(420);

    return i;
}

void therm_write_bit(uint8_t bit)
{
    THERM_LOW();
    THERM_OUTPUT_MODE();
    _delay_us(5);

    if (bit) THERM_INPUT_MODE();
    _delay_us(56);

    THERM_INPUT_MODE();
}

uint8_t therm_read_bit(void)
{
    THERM_OUTPUT_MODE();
    THERM_LOW();
    _delay_us(1);

    THERM_INPUT_MODE();
    _delay_us(14);

    uint8_t i = (THERM_PIN & (1 << THERM_DQ)) ? 1 : 0;

    _delay_us(45);

    return i;
}

void therm_write(uint8_t data)
{
    for (int i=0; i<8; i++)
    {
        therm_write_bit(data & (1 << i));
    }
}

uint8_t therm_read(void)
{
    uint8_t data = 0;

    for (int i=0; i<8; i++) {
        data |= therm_read_bit() << i;
    }

    return data;
}

/* ---- Commands ---- */

#define SEARCH_ROM      0xf0
#define READ_ROM        0x33
#define MATCH_ROM       0x55
#define SKIP_ROM        0xcc
#define ALARM_SEARCH    0xec

#define CONVERT_T       0x44
#define WRITE_SCRATCH   0x4e
#define READ_SCRATCH    0xbe
#define COPY_SCRATCH    0x48
#define RECALL_E2       0xb8
#define READ_POWER_SUPPLY 0xb4

void therm_command(uint8_t command)
{
    therm_write(command);
}

bool therm_read_temp(temp_info* tinfo)
{
  // NOTE: there may be timing issues here, as leaving debug on seems to fix it
    debug("Reading temperature...");

    if (therm_reset()) {
        return false;
    }

    therm_command(SKIP_ROM);
    therm_command(CONVERT_T);

    while (!therm_read_bit());

    debug("Temp conversion finished, reading...");   

    if (therm_reset()) {
        return false;
    }

    therm_command(SKIP_ROM);
    therm_command(READ_SCRATCH);

    uint16_t temp = 0;
    uint8_t crc = 0x0; 

    uint8_t next_byte = therm_read();
    crc = _crc_ibutton_update(crc, next_byte);
    temp = next_byte;

    next_byte = therm_read();
    crc = _crc_ibutton_update(crc, next_byte);

    temp |= (next_byte & 0x0F) << 8;  // disregard the top 4 bits, and shift into MSD

    crc = _crc_ibutton_update(crc, therm_read());
    crc = _crc_ibutton_update(crc, therm_read());
    crc = _crc_ibutton_update(crc, therm_read());
    crc = _crc_ibutton_update(crc, therm_read());
    crc = _crc_ibutton_update(crc, therm_read());
    crc = _crc_ibutton_update(crc, therm_read());
    uint8_t crc_from_sensor = therm_read();

    if (crc_from_sensor != crc) {
      therm_reset();
      debug("Failed to match crc");
      return false;
    }

    if (therm_reset()) { 
      debug("Failed to match crc");
      return false;
    }

    tinfo->major =  (temp & 0xFF0) >> 4;
    tinfo->minor = (temp & 0xF) * 625;
    tinfo->raw = temp;

    return true;
}
