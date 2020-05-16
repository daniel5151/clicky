#include "bootloader.h"
#include "ipodhw.h"
#include "minilibc.h"

#define LCD_DATA 0x10
#define LCD_CMD  0x08

#define R_START_OSC             0x00
#define R_DRV_OUTPUT_CONTROL    0x01
#define R_DRV_WAVEFORM_CONTROL  0x02
#define R_POWER_CONTROL         0x03
#define R_CONTRAST_CONTROL      0x04
#define R_ENTRY_MODE            0x05
#define R_ROTATION              0x06
#define R_DISPLAY_CONTROL       0x07
#define R_CURSOR_CONTROL        0x08
#define R_HORIZONTAL_CURSOR_POS 0x0b
#define R_VERTICAL_CURSOR_POS   0x0c
#define R_1ST_SCR_DRV_POS       0x0d
#define R_2ND_SCR_DRV_POS       0x0e
#define R_RAM_WRITE_MASK        0x10
#define R_RAM_ADDR_SET          0x11
#define R_RAM_DATA              0x12

static ipod_t    ipod;

static int ipod_is_pp5022(void) {
  return (inl(0x70000000) << 8) >> 24 == '2';
}

struct sysinfo_t {
        unsigned IsyS;  /* == "IsyS" */
        unsigned len;
        char BoardHwName[16];
        char pszSerialNumber[32];
        char pu8FirewireGuid[16];
        unsigned boardHwRev;
        unsigned bootLoaderImageRev;
        unsigned diskModeImageRev;
        unsigned diagImageRev;
        unsigned osImageRev;
        unsigned iram_perhaps;
        unsigned Flsh;
        unsigned flash_zero;
        unsigned flash_base;
        unsigned flash_size;
        unsigned flash_zero2;
        unsigned Sdrm;
        unsigned sdram_zero;
        unsigned sdram_base;
        unsigned sdram_size;
        unsigned sdram_zero2;
        unsigned Frwr;
        unsigned frwr_zero;
        unsigned frwr_base;
        unsigned frwr_size;
        unsigned frwr_zero2;
        unsigned Iram;
        unsigned iram_zero;
        unsigned iram_base;
        unsigned iram_size;
        unsigned iram_zero2;
        char pad7[120];
        unsigned boardHwSwInterfaceRev;

        /* added in V3 */
        char HddFirmwareRev[10];
        unsigned short RegionCode;
        unsigned PolicyFlags;
        char ModelNumStr[16];
};

#define SYSINFO_TAG             0x40017f18
#define SYSINFO_PTR             (struct sysinfo_t **)0x40017f1c
#define SYSINFO_TAG_PP5022      0x4001ff18
#define SYSINFO_PTR_PP5022      (struct sysinfo_t **)0x4001ff1c

static void ipod_set_sysinfo(void) {
  struct sysinfo_t ** sysinfo_ptr;
  if (ipod_is_pp5022()) {
    sysinfo_ptr = SYSINFO_PTR_PP5022;
  } else {
    sysinfo_ptr = SYSINFO_PTR;
  }
  if ( (*sysinfo_ptr)->IsyS == *(unsigned *)"IsyS" ) {
    if ((*sysinfo_ptr)->len == 0xf8) {
      ipod.hw_rev = (*sysinfo_ptr)->sdram_zero2; 
    } else {
      ipod.hw_rev = (*sysinfo_ptr)->boardHwSwInterfaceRev; 
    }
  }
  ipod.hw_ver = ipod.hw_rev >> 16;
}

/* get current usec counter */
unsigned long timer_get_current(void) {
  return inl(ipod.rtc);
}

/* check if number of useconds has passed */
int timer_passed(unsigned long clock_start, int usecs) {
  unsigned long clock;
  clock = inl(ipod.rtc);
  return (clock - clock_start) >= usecs;
}

void ipod_reboot ()
{
  if (ipod.hw_rev >= 0x40000) {
    outl(inl(0x60006004) | 0x4, 0x60006004);
  } else {
    outl(inl(0xcf005030) | 0x4, 0xcf005030);
  }
  /* We never exit this function */
  for(;;) {}
}



#define IPOD_I2C_CTRL	(ipod_i2c_base+0x00)
#define IPOD_I2C_ADDR	(ipod_i2c_base+0x04)
#define IPOD_I2C_DATA0	(ipod_i2c_base+0x0c)
#define IPOD_I2C_DATA1	(ipod_i2c_base+0x10)
#define IPOD_I2C_DATA2	(ipod_i2c_base+0x14)
#define IPOD_I2C_DATA3	(ipod_i2c_base+0x18)
#define IPOD_I2C_STATUS	(ipod_i2c_base+0x1c)

/* IPOD_I2C_CTRL bit definitions */
#define IPOD_I2C_SEND	0x80

/* IPOD_I2C_STATUS bit definitions */
#define IPOD_I2C_BUSY	(1<<6)

static unsigned ipod_i2c_base;

static int ipod_i2c_wait_not_busy(void)
{
  long start = timer_get_current ();
  do {
    if (!(inb(IPOD_I2C_STATUS) & IPOD_I2C_BUSY)) {
      return 0;
    }
  } while (!timer_passed (start, 100000));
  return -1;
}

static int ipod_i2c_send_bytes(unsigned int addr, unsigned int len, unsigned char *data)
{
  int data_addr;
  int i;
  if (len < 1 || len > 4) {
    return -1;
  }
  if (ipod_i2c_wait_not_busy() < 0) {
    return -1;
  }
  outb((addr << 17) >> 16, IPOD_I2C_ADDR);
  outb(inb(IPOD_I2C_CTRL) & ~0x20, IPOD_I2C_CTRL);
  data_addr = IPOD_I2C_DATA0;
  for ( i = 0; i < len; i++ ) {
    outb(*data++, data_addr);
    data_addr += 4;
  }
  outb((inb(IPOD_I2C_CTRL) & ~0x26) | ((len-1) << 1), IPOD_I2C_CTRL);
  outb(inb(IPOD_I2C_CTRL) | IPOD_I2C_SEND, IPOD_I2C_CTRL);
  return 0;
}

static int ipod_i2c_send(unsigned int addr, int data0, int data1)
{
  unsigned char data[2];
  data[0] = data0;
  data[1] = data1;
  return ipod_i2c_send_bytes(addr, 2, data);
}

static int ipod_i2c_send_byte(unsigned int addr, int data0)
{
  unsigned char data[1];
  data[0] = data0;
  return ipod_i2c_send_bytes(addr, 1, data);
}

static int ipod_i2c_read_byte(unsigned int addr, unsigned int *data)
{
  if (ipod_i2c_wait_not_busy() < 0) {
    return -1;
  }
  // clear top 15 bits, left shift 1, or in 0x1 for a read
  outb(((addr << 17) >> 16) | 0x1, IPOD_I2C_ADDR);
  outb(inb(IPOD_I2C_CTRL) | 0x20, IPOD_I2C_CTRL);
  outb(inb(IPOD_I2C_CTRL) | IPOD_I2C_SEND, IPOD_I2C_CTRL);
  if (ipod_i2c_wait_not_busy() < 0) {
    return -1;
  }
  if (data) {
    *data = inb(IPOD_I2C_DATA0);
  }
  return 0;
}

static int i2c_readbyte(unsigned int dev_addr, int addr)
{
  int data;
  ipod_i2c_send_byte(dev_addr, addr);
  ipod_i2c_read_byte(dev_addr, &data);
  return data;
}

void ipod_i2c_init(void)
{
  /* reset I2C */
  int hwver = ipod.hw_ver;
  if (hwver > 0x03) {
    ipod_i2c_base = 0x7000c000;
    if (hwver == 0x04) {
      /* GPIO port C disable port 0x10 */
      outl(inl(0x6000d008) & ~0x10, 0x6000d008);
      /* GPIO port C disable port 0x20 */
      outl(inl(0x6000d008) & ~0x20, 0x6000d008);
    }
    outl(inl(0x6000600c) | 0x1000, 0x6000600c); /* enable 12 */
    outl(inl(0x60006004) | 0x1000, 0x60006004); /* start reset 12 */
    outl(inl(0x60006004) & ~0x1000, 0x60006004);  /* end reset 12 */
    outl(0x0, 0x600060a4);
    outl(0x80 | (0 << 8), 0x600060a4);
    i2c_readbyte(0x08, 0);
  } else {
    ipod_i2c_base = 0xc0008000;
    outl(inl(0xcf005000) | 0x2, 0xcf005000);
    outl(inl(0xcf005030) | (1<<8), 0xcf005030);
    outl(inl(0xcf005030) & ~(1<<8), 0xcf005030);
  }
}

void pcf_standby_mode(void)
{
  // !TT 17Apr06: do not call the init from ipod_init_hardware(),
  //   because the changed state actually causes some Apple OS versions
  //   on some models to crash (meaning: the Apple OS does not like
  //   it when the hardware is not in a "reset" state)
  //   But calling it here is fine, because when we put the iPod to
  //   sleep, it'll get reset when it wakes up.
  ipod_i2c_init ();
  
  ipod_i2c_send(0x8, 0x8, 0x1 | (1 << 5) | (1 << 6));
  /* We never exit this function - a wakeup will reboot */
  for(;;) {}
}


void ipod_beep(int duration_ms, int period)
  // period: 40=2286Hz, 30=3024Hz, 20=4465Hz, 10=8547Hz
{
  if (ipod.hw_ver >= 4) {
    if (duration_ms == 0 && period == 0) {
      // both values 0 -> make a click
      duration_ms = 1;
      period = 20;
    }
    if (period < 0) duration_ms = 30; // default period
    if (duration_ms < 0) duration_ms = 50; // default duration
    if (duration_ms > 1000) duration_ms = 1000; // max beep duration is 1s
    outl(inl(0x70000010) & ~0xc, 0x70000010);
    outl(inl(0x6000600c) | 0x20000, 0x6000600c);  /* enable device */
    outl(0x80000000 | 0x800000 | (period & 0xffff), 0x7000a000); /* set pitch */
    int starttime = timer_get_current();
    do { } while (!timer_passed (starttime, duration_ms*1000));
    outl(0x0, 0x7000a000);  /* piezo off */
  } else {
    // !!! still missing -- need to write to serial port
  }
}


/* wait for LCD with timeout */
void lcd_wait_ready(void) {
  if ((inl(ipod.lcd_base) & ipod.lcd_busy_mask) != 0) {
    int start = timer_get_current();
    do {
      if ((inl(ipod.lcd_base) & ipod.lcd_busy_mask) == 0) break;
    } while (!timer_passed(start, 1000));
  }
}

/* send LCD data */
void lcd_send_data(int data_hi, int data_lo) {
  lcd_wait_ready();
  if( ipod.hw_ver == 0x7 ) {
    outl((inl(0x70003000) & ~0x1f00000) | 0x1700000, 0x70003000);
    outl(data_lo | (data_hi << 8) | 0x760000, 0x70003008);
  } else {
    outl(data_hi, ipod.lcd_base + LCD_DATA);
    lcd_wait_ready();
    outl(data_lo, ipod.lcd_base + LCD_DATA);
  }
}

/* send LCD command */
void lcd_prepare_cmd(int cmd) {
  lcd_wait_ready();
  if( ipod.hw_ver == 0x7) {
    outl((inl(0x70003000) & ~0x1f00000) | 0x1700000, 0x70003000);
    outl(cmd | 0x740000, 0x70003008);
  } else {
    outl(0x0, ipod.lcd_base + LCD_CMD);
    lcd_wait_ready();
    outl(cmd, ipod.lcd_base + LCD_CMD);
  }
}

void lcd_cmd_and_data_hi_lo(int cmd, int data_hi, int data_lo) {
  lcd_prepare_cmd(cmd);
  lcd_send_data(data_hi, data_lo);
}

void lcd_cmd_and_data16(int cmd, uint16 data) {
  lcd_cmd_and_data_hi_lo(cmd, data >> 8, data & 0xFF);
}

static unsigned int lcd_contrast = 0x6a;  /* default for mini2 */

int lcd_curr_contrast () // returns current contrast
{
  if (ipod.hw_ver == 7) {
    // mini 2g - can't read the contrast. we use a global that
    // defaults to 0x6a instead.
    return lcd_contrast;
  } else if (ipod.hw_ver < 6) {
    lcd_wait_ready ();
    (void) inl(ipod.lcd_base + LCD_CMD);
    lcd_wait_ready ();
    return inl(ipod.lcd_base + LCD_CMD) & 0x7F;
  }
  return 0; // newer models: no contrast support by iPL yet
}

void lcd_set_contrast(int val)
{
  int ver = ipod.hw_ver;
  if (ver < 6 || ver == 0x7) {
    if (val < 0) val = 0;
    else if (val > 127) val = 127;
    if (val != lcd_curr_contrast()) {
      if (ver == 7 || ver < 6) {
        lcd_cmd_and_data16 (R_CONTRAST_CONTROL, 0x400 | val);
      }
    }
  }
  lcd_contrast = val;
}


void ipod_set_backlight(int on) {

  if(ipod.hw_ver >= 0x4) {
    if (ipod.hw_ver == 0x5 || ipod.hw_ver == 0x6) {
      if (on) {
	/* brightness full */
	outl(0x80000000 | (0xff << 16), 0x7000a010);
	
	/* set port B03 on */
	outl(((0x100 | 1) << 3), 0x6000d824);
      } else {
	/* fades backlght off on 4g */
	/* GPO D01 disable */
	outl(inl(0x70000084) & ~0x2000000, 0x70000084);
	outl(0x80000000, 0x7000a010);
      }
    } else if (ipod.hw_ver == 0x04 || ipod.hw_ver == 0x7) {
      /* set port B03 */
      outl(((0x100 | (on ? 1 : 0)) << 3), 0x6000d824);
    } else if ( (ipod.hw_ver == 0xc) || (ipod.hw_ver == 0xb) ) {
      /* set port B03 */
      outl(((0x100 | (on ? 1 : 0)) << 3), 0x6000d824);
      /* set port L07 */
      outl(((0x100 | (on ? 1 : 0)) << 7), 0x6000d12c);
    }
  } else {
    int lcd_state;
    
    lcd_state = inl(IPOD_PP5002_LCD_BASE);
    if (on) {
      lcd_state = lcd_state | 0x2;
    }
    else {
      lcd_state = lcd_state & ~0x2;
    }
    outl(lcd_state, IPOD_PP5002_LCD_BASE);
  }
  
  if (ipod.hw_ver < 0x6 || ipod.hw_ver == 0x7) {
    if (on) {
      /* display control (1 00 0 1) */
      /* GSH=01 -> 2/3 level grayscale control */
      /* GSL=00 -> 1/4 level grayscale control */
      /* REV=0 -> don't reverse */
      /* D=1 -> display on */
      if (ipod.hw_ver < 3) {
        /* REV=1 */
        lcd_cmd_and_data_hi_lo(0x7, 0x0, 0x11 | 0x2);
      }
      else {
        lcd_cmd_and_data_hi_lo(0x7, 0x0, 0x11);
      }
    }
    else {
      /* display control (10 0 1) */
      /* GSL=10 -> 2/4 level grayscale control */
      /* REV=0 -> don't reverse */
      /* D=1 -> display on */
      lcd_cmd_and_data_hi_lo(0x7, 0x0, 0x9);
    }
  }

}


ipod_t *ipod_get_hwinfo(void) {
  return(&ipod);
}

void ipod_init_hardware(void) {
  int hw_ver;
  ipod_set_sysinfo();

  hw_ver = ipod.hw_ver;
  if( hw_ver == 0xC ) { /* Nano */
    ipod.lcd_base      = 0x70008A0C;
    ipod.lcd_busy_mask = 0x80000000;
    ipod.lcd_width     = 176;
    ipod.lcd_height    = 132;
    ipod.lcd_type      = 1;
    ipod.lcd_format    = IPOD_LCD_FORMAT_RGB565;
    ipod.rtc           = IPOD_PP5020_RTC;
    ipod.ide_base      = IPOD_PP5020_IDE_PRIMARY_BASE;
    ipod.ide_control   = IPOD_PP5020_IDE_PRIMARY_CONTROL;
    ipod.mem_base      = 0x10000000;
    ipod.mem_size      = 0x02000000;
  } else if( hw_ver == 0xB ) { /* 5th Gen  (Video) */
    ipod.lcd_base      = 0x70008A0C;
    ipod.lcd_busy_mask = 0x80000000;
    ipod.lcd_width     = 320;
    ipod.lcd_height    = 240;
    ipod.lcd_type      = 5;
    ipod.lcd_format    = IPOD_LCD_FORMAT_RGB565;
    ipod.rtc           = IPOD_PP5020_RTC;
    ipod.ide_base      = IPOD_PP5020_IDE_PRIMARY_BASE;
    ipod.ide_control   = IPOD_PP5020_IDE_PRIMARY_CONTROL;
    ipod.mem_base      = 0x10000000;
    ipod.mem_size      = 0x02000000;  /* 0x04000000 for 60GB model.. Maybe a check? */
  } else if( hw_ver == 0x6 ) { /* Photo / Color */
    ipod.lcd_base      = 0x70008A0C;
    ipod.lcd_busy_mask = 0x80000000;
    ipod.lcd_width     = 220;
    ipod.lcd_height    = 176;
    ipod.lcd_format    = IPOD_LCD_FORMAT_RGB565;
    ipod.rtc           = IPOD_PP5020_RTC;
    ipod.ide_base      = IPOD_PP5020_IDE_PRIMARY_BASE;
    ipod.ide_control   = IPOD_PP5020_IDE_PRIMARY_CONTROL;
    ipod.mem_base      = 0x10000000;
    ipod.mem_size      = 0x02000000;

    if(ipod.hw_rev == 0x60000) ipod.lcd_type = 0;
    else { /* Detect the LCD type */
      int gpio_a01, gpio_a04;
	    
      /* A01 */
      gpio_a01 = (inl(0x6000D030) & 0x2) >> 1;
      /* A04 */
      gpio_a04 = (inl(0x6000D030) & 0x10) >> 4;
      
      if (((gpio_a01 << 1) | gpio_a04) == 0 || ((gpio_a01 << 1) | gpio_a04) == 2) {
        ipod.lcd_type = 0;
      } else {
        ipod.lcd_type = 1;
      }
    }
  } else if( hw_ver == 0x5 ) { /* 4G */
    ipod.lcd_base      = IPOD_PP5020_LCD_BASE;
    ipod.lcd_busy_mask = 0x8000;
    ipod.lcd_width     = 160;
    ipod.lcd_height    = 128;
    ipod.lcd_type      = 0;
    ipod.lcd_format    = IPOD_LCD_FORMAT_2BPP;
    ipod.rtc           = IPOD_PP5020_RTC;
    ipod.ide_base      = IPOD_PP5020_IDE_PRIMARY_BASE;
    ipod.ide_control   = IPOD_PP5020_IDE_PRIMARY_CONTROL;
    ipod.mem_base      = 0x10000000;
    ipod.mem_size      = 0x02000000;
  } else if( (hw_ver == 0x4) || 
	     (hw_ver == 0x7) ) { /* Mini / Mini 2G */
    ipod.lcd_base      = IPOD_PP5020_LCD_BASE;
    ipod.lcd_busy_mask = 0x8000;
    ipod.lcd_width     = 136; // yes, in theory, we have 138 pixels, but we use only 136. do not change this without testing it!
    ipod.lcd_height    = 110;
    ipod.lcd_type      = 0;
    ipod.lcd_format    = IPOD_LCD_FORMAT_2BPP;
    ipod.rtc           = IPOD_PP5020_RTC;
    ipod.ide_base      = IPOD_PP5020_IDE_PRIMARY_BASE;
    ipod.ide_control   = IPOD_PP5020_IDE_PRIMARY_CONTROL;
    ipod.mem_base      = 0x10000000;
    ipod.mem_size      = 0x02000000;
  } else if( hw_ver < 0x4 ) { /* 3G / 2G / 1G */
    ipod.lcd_base      = IPOD_PP5002_LCD_BASE;
    ipod.lcd_busy_mask = 0x8000;
    ipod.lcd_width     = 160;
    ipod.lcd_height    = 128;
    ipod.lcd_type      = 0;
    ipod.lcd_format    = IPOD_LCD_FORMAT_2BPP;
    ipod.rtc           = IPOD_PP5002_RTC;
    ipod.ide_base      = IPOD_PP5002_IDE_PRIMARY_BASE;
    ipod.ide_control   = IPOD_PP5002_IDE_PRIMARY_CONTROL;
    ipod.mem_base      = 0x28000000;
    ipod.mem_size      = 0x02000000;
  }

  // determine the Fast RAM start and size
  ipod.iram_base = 0x40000000;
  if (ipod_is_pp5022 ()) {
      ipod.iram_full_size = 0x20000;  // PP5022 has 128MB of internal RAM
  } else {
      ipod.iram_full_size = 0x18000;  // PP5020 and PP5002 have 96MB of internal RAM
  }
  ipod.iram_user_end = ipod.iram_base + ipod.iram_full_size - 0x100; // last 256 bytes appear to be used for special things by the Flash ROM

  ipod.lcd_is_grayscale = (ipod.lcd_format == IPOD_LCD_FORMAT_2BPP);
}
