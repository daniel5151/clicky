#ifndef _FB_H_
#define _FB_H_

#include "bootloader.h"

// commonly used colors:
#define BLACK 0x0000
#define WHITE 0xFFFF

void fb_init(void);

void fb_update(uint16 *x);
void fb_cls(uint16 *x,uint16 val);
uint16 fb_rgb(int r, int g, int b); // takes values between 0 and 255
void fb_rgbsplit (uint16 rgb, uint8 *r, uint8 *g, uint8 *b);

#endif
