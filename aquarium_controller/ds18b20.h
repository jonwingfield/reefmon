#ifndef __ds18b20_h__
#define __ds18b20_h__

#define THERM_PORT PORTD
#define THERM_DDR  DDRD
#define THERM_DD   DDD7
#define THERM_PIN  PIND
#define THERM_DQ   PD7

typedef struct {
	uint16_t major;  /* left side of decimal (could be uint8_t) */
	uint16_t minor;  /* right side of decimal, 4 digits, but res only to .0625C */
  uint16_t raw;
} temp_info;

/* Returns the temperature in deg Celcius */
bool therm_read_temp(temp_info* tinfo);

#endif
