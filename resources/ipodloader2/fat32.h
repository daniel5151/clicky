#ifndef _FAT32_H_
#define _FAT32_H_

#include "bootloader.h"

typedef struct {
  uint32 cluster;
  uint32 length;
  uint32 opened;
  uint32 position;
} fat32_file;

void fat32_newfs(uint8 part,uint32 offset);

#endif
