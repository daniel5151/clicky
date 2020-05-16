#include "bootloader.h"
#include "console.h"
#include "fb.h"
#include "menu.h"
#include "minilibc.h"
#include "config.h"
#include "ipodhw.h"
#include "lockicon.h"

static struct {
  ipod_t *ipod;
  int    numItems;
  char   *string[MAX_MENU_ITEMS];
  int    x,y,w,h,fh;
  config_t *conf;
} menu;

/* Clears the screen to a nice black to blue gradient */
void menu_cls(uint16 *fb) {
  int x,y;
  if (menu.ipod->lcd_is_grayscale) {
    fb_cls (fb, 0);
  } else if (!menu.conf->usegradient) {
    fb_cls (fb, menu.conf->bgcolor);
  } else {
    int h = menu.ipod->lcd_height;
    int w = menu.ipod->lcd_width;
    uint8 r, g, b;
    fb_rgbsplit (menu.conf->bgcolor, &r, &g, &b);
    for (y=menu.ipod->lcd_height-1; y >= 0; y--) {
      uint16 pix = fb_rgb ((int)r*y/h, (int)g*y/h, (int)b*y/h);
      int ofs = y * w;
      x = w;
      while (x--) {
        fb[ofs++] = pix;
      }
    }
  }
}

static void menu_drawicon (uint16 *fb, int top, int left, int w, int h, uint16 *icondata, int transparent) {
  //int bw = menu.ipod->lcd_is_grayscale;
  int x, y, lcd_w = menu.ipod->lcd_width;
  int ofs = top * lcd_w + left;
  for (y=0; y<h; y++) {
    for (x=0; x<w; x++) {
      int d = *icondata++;
      if (d != transparent) {
        // doesn't look good: if (bw) d = 0xffff;
        fb[ofs+x] = d;
      }
    }
    ofs += lcd_w;
  }
}

static void menu_drawlock (uint16 *fb) {
  int w = lock_image.width;
  int h = lock_image.height;
  int t = menu.y + ((menu.h - h) >> 1);
  int l = menu.x + ((menu.w - w) >> 1);
  menu_drawicon (fb, t, l, w, h, lock_image.data, 0);
}

void menu_drawrect(uint16 *fb, int x1, int y1, int x2, int y2, uint16 color) {
  int x,y;
  if (y1 < 0) y1 = 0;
  if (y2 >= menu.ipod->lcd_height) y2 = menu.ipod->lcd_height-1;
  for(y=y1;y<=y2;y++) {
    int ofs = y * menu.ipod->lcd_width;
    for(x=x1;x<=x2;x++) {
      fb[ofs + x] = color;
    }
  }
}

void menu_hline (uint16 *fb, int x1, int x2, int y, uint16 color) {
  if (y >= 0 && y < menu.ipod->lcd_height) {
    int x;
    int ofs = y * menu.ipod->lcd_width;
    for (x=x1; x<=x2; x++) {
      fb[ofs + x] = color;
    }
  }
}

void menu_vline (uint16 *fb, int x, int y1, int y2, uint16 color) {
  int y;
  if (y1 < 0) y1 = 0;
  if (y2 > menu.ipod->lcd_height) y2 = menu.ipod->lcd_height;
  int ofs = y1 * menu.ipod->lcd_width + x;
  for (y=y1; y<=y2; y++) {
    fb[ofs] = color;
    ofs += menu.ipod->lcd_width;
  }
}

void menu_frame (uint16 *fb, int x1, int y1, int x2, int y2, uint16 color) {
  menu_hline(fb,x1,x2,y1,color);
  menu_hline(fb,x1,x2,y2,color);
  menu_vline(fb,x1,y1,y2,color);
  menu_vline(fb,x2,y1,y2,color);
}

static void menu_recenter() {
  int i;

  menu.w = 0;
  for(i = 0; i < menu.numItems; i++) {
    if(menu.w < (mlc_strlen (menu.string[i]) << 3))
      menu.w = mlc_strlen (menu.string[i]) << 3;
  }
  menu.w += 6;
  menu.fh = 16;
  menu.h = menu.numItems * 20;
  if(menu.h > (menu.ipod->lcd_height - 50)) {
    menu.fh = 8;
    menu.h = menu.numItems * 12;
  }
  if(menu.w < (menu.ipod->lcd_width*3/5))
    menu.w = menu.ipod->lcd_width*3/5;
  if(menu.h < (menu.ipod->lcd_height*2/5))
    menu.h = menu.ipod->lcd_height*2/5;
  menu.x = (menu.ipod->lcd_width - menu.w) >> 1;
  menu.y = ((menu.ipod->lcd_height - menu.h - (menu.fh + 6)) >> 1) + menu.fh + 6;
}

void menu_init() {
  menu.ipod = ipod_get_hwinfo();
  menu.conf = config_get ();
  menu.numItems = 0;
}

void menu_additem(char *text) {
  if (menu.numItems < MAX_MENU_ITEMS) {
    menu.string[menu.numItems++] = text;
    menu_recenter();
    if (menu.h > menu.ipod->lcd_height - (menu.fh+6)) {
      // too many items
      menu.numItems--;
      menu_recenter();
    }
  }
}

void menu_drawprogress(uint16 *fb,uint8 completed) {
  uint16 pbarWidth;
  static char tmpBuff[40];

  pbarWidth = (menu.ipod->lcd_width - 20);

  menu_cls(fb);

  menu_drawrect( fb,10,(menu.ipod->lcd_height>>1)-5,
		 10+pbarWidth,(menu.ipod->lcd_height>>1)+5,
                 BLACK );
  menu_drawrect( fb,10,(menu.ipod->lcd_height>>1)-5,
                 10+(completed*pbarWidth)/255,(menu.ipod->lcd_height>>1)+5,
                 (menu.ipod->lcd_is_grayscale? WHITE : menu.conf->hicolor) );

  console_putsXY(1,1,tmpBuff);
  fb_update(fb);
}

void menu_redraw(uint16 *fb, int selectedItem, char *title, char *countDown, int drawLock) {
  int i;
  int line_height = menu.fh + 4;
  uint16 fg, bg;
  uint8 tp;

  const uint8 *menu_font = (menu.fh == 16)? font_large : font_medium;
  const uint8 *prev_font = console_currentfont ();
  console_setfont (menu_font);

  menu_cls(fb);

  console_getcolor(&fg, &bg, &tp);
  console_setcolor(WHITE, BLACK, 1);

  console_putsXY(2,2,title);
  console_putsXY(menu.ipod->lcd_width-2-mlc_strlen(countDown)*font_width,2,countDown);

  menu_hline(fb, 2, menu.ipod->lcd_width-2, menu.fh+2, WHITE);
  menu_frame(fb, menu.x-2, menu.y-2, menu.x+menu.w+1, menu.y+menu.h+1, WHITE);

  for (i=0; i<menu.numItems; i++) {
    if (i==selectedItem) {
      uint16 bg;
      if (menu.ipod->lcd_is_grayscale) {
        bg = WHITE;
        console_setcolor (BLACK, bg, 0);
      } else {
        bg = menu.conf->hicolor;
        console_setcolor(WHITE, bg, 0);
      }
      menu_drawrect(fb, menu.x, menu.y+i*line_height, menu.x+menu.w-1, menu.y+(i+1)*line_height-1, bg);
    } else {
      console_setcolor(WHITE, BLACK, 1);
    }
    console_putsXY(menu.x+2, menu.y+i*line_height+2, menu.string[i]);
  }
  
  if (drawLock) {
    menu_drawlock (fb);
  }

  console_setcolor(fg, bg, tp);
  console_setfont (prev_font); // reset font
}
