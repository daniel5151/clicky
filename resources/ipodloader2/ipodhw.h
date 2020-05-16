#ifndef _IPODHW_H_
#define _IPODHW_H_

#include "bootloader.h"

#define IPOD_PP5002_RTC 0xCF001110
#define IPOD_PP5020_RTC 0x60005010
#define IPOD_PP5002_LCD_BASE    0xC0001000
#define IPOD_PP5020_LCD_BASE    0x70003000

#define IPOD_PP5002_IDE_PRIMARY_BASE         0xC00031E0
#define IPOD_PP5002_IDE_PRIMARY_CONTROL      0xC00033F8
#define IPOD_PP5020_IDE_PRIMARY_BASE         0xC30001E0
#define IPOD_PP5020_IDE_PRIMARY_CONTROL      0xC30003F8

#define IPOD_LCD_FORMAT_2BPP   0x00
#define IPOD_LCD_FORMAT_RGB565 0x01

typedef struct {
  uint32 hw_rev;
  uint32 lcd_base, lcd_busy_mask;
  uint32 rtc;
  uint32 ide_base, ide_control;
  uint32 mem_base, mem_size;
  uint32 iram_base, iram_full_size, iram_user_end;
  int32 lcd_height, lcd_width;
  int16 hw_ver;        // = hw_rev>>16
  uint8 lcd_format;
  uint8 lcd_type;
  uint8 lcd_is_grayscale;
} ipod_t;

void    ipod_init_hardware(void);
ipod_t *ipod_get_hwinfo(void);

#define TIMER_SECOND (1000000)
#define TIMER_MINUTE (60000000)

unsigned long timer_get_current(void);
int timer_passed(unsigned long clock_start, int usecs);
void lcd_wait_ready(void);
void lcd_prepare_cmd(int cmd);
void lcd_send_data(int data_hi, int data_lo);
void lcd_cmd_and_data16(int cmd, uint16 data);
void lcd_cmd_and_data_hi_lo(int cmd, int data_hi, int data_lo);
void lcd_set_contrast(int val);
int lcd_curr_contrast ();
void ipod_set_backlight(int on);
void ipod_reboot (void);
void pcf_standby_mode(void);
void ipod_i2c_init(void);
void ipod_beep(int duration_ms, int period);

#endif
