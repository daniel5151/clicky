#ifndef _FWFS_H_
#define _FWFS_H_

#include "bootloader.h"

typedef struct {
  uint32 magic;     /* [hi] */
  uint32 bl_table;  /* Start location of bootloader table */
  uint16 ext_head;  /* Start location of extended header */
  uint16 version;   /* Firmware format version (2=Pre 4G, 3=Post 4G) */ 
} fwfs_header_t;

typedef struct {
  uint32 dev;
  uint32 type;
  uint32 id;
  uint32 devOffset;
  uint32 len;
  uint32 addr;
  uint32 entryOffset;
  uint32 chksum;
  uint32 vers;
  uint32 loadaddr;
} fwfs_image_t;

typedef struct {
  uint32 devOffset;
  uint32 length;
  uint32 chksum;

  uint32 position;
} fwfs_file;

void fwfs_newfs(uint8 part,uint32 offset);

#endif
