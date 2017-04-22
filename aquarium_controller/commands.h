#ifndef AQ_COMMANDS_H
#define AQ_COMMANDS_H

#include "SMBSlave.h"

#define AQ_CMD_SETCHANNELS            0x11
#define AQ_CMD_GET_TEMP               0x12
#define AQ_CMD_GET_DEPTH              0x13
#define AQ_CMD_GET_AIR_TEMP_HUMIDITY  0x14

void ProcessMessage(SMBData* smb);

typedef struct {
  uint8_t channels[6];
} AQ_Commands;

#endif


