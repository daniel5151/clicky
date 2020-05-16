#ifndef _ATA2_H_
#define _ATA2_H_

#include "bootloader.h"

uint32 ata_init(void);
void   ata_exit(void);
void   ata_identify(void);
void   ata_find_transfermode(void);
uint8  ata_get_drivetype (void);
int    ata_readblock(void *dst, uint32 sector);	// this read get cached
int    ata_readblocks(void *dst,uint32 sector,uint32 count);	// these reads get cached
int    ata_readblocks_uncached(void *dst,uint32 sector,uint32 count);	// these reads are uncached
void   ata_standby (int cmd_variation);

#endif
