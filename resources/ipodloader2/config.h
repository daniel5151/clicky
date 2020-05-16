#ifndef _CONFIG_H_
#define _CONFIG_H_

#include "bootloader.h"

#define CONFIG_IMAGE_BINARY  0x00
#define CONFIG_IMAGE_SPECIAL 0x01
#define CONFIG_IMAGE_ROCKBOX 0x02

typedef struct {
  uint32 type;
  char  *title;
  char  *path;
} config_image_t;

typedef struct {
  config_image_t *image;
  int16 timeout;
  int16 def;       // default item index in menu, 1-based
  int16 items;
  int16 backlight;
  int16 contrast;
  uint16 debug;
  uint16 usegradient;
  uint16 bgcolor;
  uint16 hicolor;
  uint16 beep_time; // in ms
  uint16 beep_period;
  int16 ata_standby_code;
} config_t;

void      config_init(void);
config_t *config_get(void);

#endif
