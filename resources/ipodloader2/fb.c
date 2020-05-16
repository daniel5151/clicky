#include "bootloader.h"
#include "ipodhw.h"

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

static ipod_t *ipod;

static void lcd_send_lo(int v) {
  lcd_wait_ready();
  outl(v | 0x80000000, ipod->lcd_base);
}

static void lcd_send_hi(int v) {
  lcd_wait_ready();
  outl(v | 0x81000000, ipod->lcd_base);
}

static void lcd_cmd_data(int cmd, int data) {
  if( ipod->lcd_type == 0) {
    lcd_send_lo(cmd);
    lcd_send_lo(data);
  } else {
    lcd_send_lo(0x0);
    lcd_send_lo(cmd);
    lcd_send_hi((data >> 8) & 0xff);
    lcd_send_hi(data & 0xff);
  }
}

static void lcd_bcm_write32(unsigned address, unsigned value) {
  /* write out destination address as two 16bit values */
  outw(address, 0x30010000);
  outw((address >> 16), 0x30010000);

  /* wait for it to be write ready */
  while ((inw(0x30030000) & 0x2) == 0);

  /* write out the value low 16, high 16 */
  outw(value, 0x30000000);
  outw((value >> 16), 0x30000000);
}

static void lcd_bcm_setup_rect(unsigned cmd, unsigned start_horiz, unsigned start_vert, unsigned max_horiz, unsigned max_vert, unsigned count) {
  lcd_bcm_write32(0x1F8, 0xFFFA0005);
  lcd_bcm_write32(0xE0000, cmd);
  lcd_bcm_write32(0xE0004, start_horiz);
  lcd_bcm_write32(0xE0008, start_vert);
  lcd_bcm_write32(0xE000C, max_horiz);
  lcd_bcm_write32(0xE0010, max_vert);
  lcd_bcm_write32(0xE0014, count);
  lcd_bcm_write32(0xE0018, count);
  lcd_bcm_write32(0xE001C, 0);
}

static unsigned lcd_bcm_read32(unsigned address) {
  while ((inw(0x30020000) & 1) == 0);
  /* write out destination address as two 16bit values */
  outw(address, 0x30020000);
  outw((address >> 16), 0x30020000);
  /* wait for it to be read ready */
  while ((inw(0x30030000) & 0x10) == 0);
  /* read the value */
  return inw(0x30000000) | inw(0x30000000) << 16;
}

static void lcd_bcm_finishup(void) {
  unsigned data; 
  outw(0x31, 0x30030000); 
  lcd_bcm_read32(0x1FC);
  do {
    data = lcd_bcm_read32(0x1F8);
  } while (data == 0xFFFA0005 || data == 0xFFFF);
  lcd_bcm_read32(0x1FC);
}


static uint8 LUMA565(uint16 val) {
  uint16 calc; 
  calc  = (val>>11)<<3;
  calc += ((val>>5)&0x3F)<<2;
  calc += (val&0x1F)<<3;
  calc = calc / 3;
  if (calc > 0xFF) calc = 0xFF;
  return calc;
}

uint16 fb_rgb(int r, int g, int b) {
  uint16 rgb;
  if (r < 0) r = 0;
  if (r > 255) r = 255;
  if (g < 0) g = 0;
  if (g > 255) g = 255;
  if (b < 0) b = 0;
  if (b > 255) b = 255;
  rgb = ((r >> 3) << 11) + ((g >> 2) << 5) + (b >> 3);
  return rgb;
}

void fb_rgbsplit (uint16 rgb, uint8 *r, uint8 *g, uint8 *b)
  // inverse of fb_rgb()
{
  *r = (rgb>>11)<<3;
  *g = ((rgb>>5)&0x3F)<<2;
  *b = (rgb&0x1F)<<3;
}


static void fb_565_bitblt(uint16 *x, int sx, int sy, int mx, int my) {
  int startx = sy;
  int starty = sx;
  int height = (my - sy);
  int width  = (mx - sx);
  int rect1, rect2, rect3, rect4;
  
  unsigned short *addr = x;
  
  /* calculate the drawing region */
  if( ipod->hw_ver != 0x6) {
    rect1 = starty;                 /* start horiz */
    rect2 = startx;                 /* start vert */
    rect3 = (starty + width) - 1;   /* max horiz */
    rect4 = (startx + height) - 1;  /* max vert */
  } else {
    rect1 = startx;                 /* start vert */
    rect2 = (ipod->lcd_width - 1) - starty;       /* start horiz */
    rect3 = (startx + height) - 1;  /* end vert */
    rect4 = (rect2 - width) + 1;            /* end horiz */
  }
  
  /* setup the drawing region */
  if( ipod->lcd_type == 0) {
    lcd_cmd_data(0x12, rect1);      /* start vert */
    lcd_cmd_data(0x13, rect2);      /* start horiz */
    lcd_cmd_data(0x15, rect3);      /* end vert */
    lcd_cmd_data(0x16, rect4);      /* end horiz */
  } else if( ipod->lcd_type != 5 ) {
    /* swap max horiz < start horiz */
    if (rect3 < rect1) {
      int t;
      t = rect1;
      rect1 = rect3;
      rect3 = t;
    }
    
    /* swap max vert < start vert */
    if (rect4 < rect2) {
      int t;
      t = rect2;
      rect2 = rect4;
      rect4 = t;
    }

    /* max horiz << 8 | start horiz */
    lcd_cmd_data(0x44, (rect3 << 8) | rect1);
    /* max vert << 8 | start vert */
    lcd_cmd_data(0x45, (rect4 << 8) | rect2);
    
    if( ipod->hw_ver == 0x6) {
      /* start vert = max vert */
      rect2 = rect4;
    }
    
    /* position cursor (set AD0-AD15) */
    /* start vert << 8 | start horiz */
    lcd_cmd_data(0x21, (rect2 << 8) | rect1);
    
    /* start drawing */
    lcd_send_lo(0x0);
    lcd_send_lo(0x22);
  } else { /* 5G */
    unsigned count = (width * height) << 1;
    lcd_bcm_setup_rect(0x34, rect1, rect2, rect3, rect4, count);
  }
  
  addr += startx * ipod->lcd_width + starty;
  
  while (height > 0) {
    int x, y;
    int h, pixels_to_write;

    if( ipod->lcd_type != 5 ) {
      pixels_to_write = (width * height) * 2;
      
      /* calculate how much we can do in one go */
      h = height;
      if (pixels_to_write > 64000) {
	h = (64000/2) / width;
	pixels_to_write = (width * h) * 2;
      }
      
      outl(0x10000080, 0x70008a20);
      outl((pixels_to_write - 1) | 0xc0010000, 0x70008a24);
      outl(0x34000000, 0x70008a20);
    } else {
      h = height;
      outw ((0xE0020 & 0xffff), 0x30010000);
      outw ((0xE0020 >> 16), 0x30010000);
      while ((inw (0x30030000) & 0x2) == 0);
    }

    /* for each row */
    for (y = 0; y < h; y++) {
      /* for each column */
      for (x = 0; x < width; x += 2) {
        /* output 2 pixels */
	if( ipod->lcd_type != 5 ) {
          unsigned two_pixels;
	  two_pixels = ( ((addr[0]&0xFF)<<8) | ((addr[0]&0xFF00)>>8) ) | 
	               ((((addr[1]&0xFF)<<8) | ((addr[1]&0xFF00)>>8) )<<16);
	  while ((inl(0x70008a20) & 0x1000000) == 0);
	  outl(two_pixels, 0x70008b00);
          addr += 2;
	} else {
	  outw (*addr++, 0x30000000);
          outw (*addr++, 0x30000000);
	}
      }
      addr += ipod->lcd_width - width;
    }

    if (ipod->lcd_type != 5) {
      while ((inl(0x70008a20) & 0x4000000) == 0);
      outl(0x0, 0x70008a24);
      height = height - h;
    } else {
      height = 0;
    }
  }

  if (ipod->lcd_type == 5) {
    lcd_bcm_finishup();
  }
}

static void fb_2bpp_bitblt(uint16 *fb, int sx, int sy, int mx, int my) {
  int y;

  sx >>= 3;
  mx >>= 3;
  
  for ( y = sy; y < my; y++ ) {
    int x;

    lcd_cmd_and_data16(R_RAM_ADDR_SET, (y << 5) + 20); // sets the cursor
    lcd_prepare_cmd(R_RAM_DATA); // intro for the data to come
    
    for ( x = sx; x < mx; x++ ) {
      uint16 pix = 0;

      /* RGB565 to 2BPP downsampling */
      for (unsigned i = 0; i < 8; ++i) {
        uint8 v = LUMA565( *fb++ ) >> 6;
        pix = (pix << 2) | v;
      }

      /* send 2 bytes (8 pixels) */
      lcd_send_data(pix >> 8, pix & 0xFF);
    }
  }
}


void fb_update(uint16 *x) {
  if( ipod->lcd_is_grayscale ) 
    fb_2bpp_bitblt(x,0,0,ipod->lcd_width,ipod->lcd_height);
  else
    fb_565_bitblt(x,0,0,ipod->lcd_width,ipod->lcd_height);
}


void fb_cls(uint16 *x,uint16 val) {
  uint32 i, n = (ipod->lcd_width*ipod->lcd_height);
  for(i=0;i<n;i++) {
    x[i] = val;
  }
}


void fb_init(void) {

  int hw_ver;
  ipod = ipod_get_hwinfo();
  hw_ver = ipod->hw_ver;

  if (hw_ver == 0x4 || hw_ver == 0x7) {
    /* driver output control - 160x112 (ipod mini) */
    lcd_cmd_and_data_hi_lo(0x1, 0x1, 0xd);
  } else if (hw_ver < 0x4 || hw_ver == 0x5) {
    /* driver output control - 160x128 */
    lcd_cmd_and_data_hi_lo(0x1, 0x0, 0xf);
  }
  
  /* ID=1 -> auto decrement address counter */
  /* AM=00 -> data is continuously written in parallel */
  /* LG=00 -> no logical operation */
  if (hw_ver < 0x6 || hw_ver == 0x7) {
    lcd_cmd_and_data_hi_lo(0x5, 0x0, 0x00);
  }
  
  if (hw_ver == 0x5 || hw_ver == 0x6) {
    outl(inl(0x6000d004) | 0x4, 0x6000d004); /* B02 enable */
    outl(inl(0x6000d004) | 0x8, 0x6000d004); /* B03 enable */
    outl(inl(0x70000084) | 0x2000000, 0x70000084); /* D01 enable */
    outl(inl(0x70000080) | 0x2000000, 0x70000080); /* D01 =1 */
    outl(inl(0x6000600c) | 0x20000, 0x6000600c);    /* PWM enable */
  }

  #if YOU_WANT_TO_SCREW_UP_THE_COLORS_IN_RETAILOS
    if( (ipod->hw_ver == 0x6) && (ipod->lcd_type == 0) ) {
      lcd_cmd_data(0xef,0x0);
      lcd_cmd_data(0x1,0x0);
      lcd_cmd_data(0x80,0x1);
      lcd_cmd_data(0x10,0x8);
      lcd_cmd_data(0x18,0x6);
      lcd_cmd_data(0x7e,0x4);
      lcd_cmd_data(0x7e,0x5);
      lcd_cmd_data(0x7f,0x1);
    }
  #endif
}
