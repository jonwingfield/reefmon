#ifndef __rht03_h__
#define __rht03_h__

#define RHT03_PORT PORTC
#define RHT03_DDR  DDRC
#define RHT03_PIN  PINC
#define RHT03_DQ   PC3

typedef struct 
{
	uint16_t temp;
	uint16_t humidity;
} temp_humidity_info;

/* reads temp/humidity info from RHT_03 and populates the struct provided */
bool rht03_temp_and_humidity(temp_humidity_info* info);

#endif
