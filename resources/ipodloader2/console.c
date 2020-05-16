#include "bootloader.h"
#include "fb.h"
#include "console.h"
#include "minilibc.h"
#include "ipodhw.h"


#include "fontlarge.h"
#include "fontmedium.h"
// not used currently: #include "fontsmall.h"


int font_lines;
int font_height, font_width;
int console_printcount = 0;

static const uint8 *fontdata;

static struct {
  struct {
    uint16 x,y;
  } cursor;
  struct {
    uint16 w,h;
  } dimensions;
  uint16 *fb;

  uint16  fgcolor,bgcolor;
  uint8   transparent;
  char    cls_pending;
  char    scroll_pending;
  char    scrollMode;

  ipod_t *ipod;
} console;

void console_setcolor(uint16 fg, uint16 bg, uint8 transparent) {
  console.fgcolor     = fg;
  console.bgcolor     = bg;
  console.transparent = transparent;
}

void console_getcolor(uint16 *fg, uint16 *bg, uint8 *transparent) {
  *fg = console.fgcolor;
  *bg = console.bgcolor;
  *transparent = console.transparent;
}

static int console_suppress_fbupdate_cnt = 0;

int console_suppress_fbupdate (int modify) {
  // pass 1 to suppress calls to fb_update with linefeeds, pass -1 to undo it again
  // pass 0 to inquire the current value
  // once the counter goes back to 0, a fb_update is performed
  console_suppress_fbupdate_cnt += modify;
  if (!console_suppress_fbupdate_cnt) {
    fb_update (console.fb);
  }
  return console_suppress_fbupdate_cnt;
}

void console_home()
{
  console.cursor.x = 0;
  console.cursor.y = 0;
  console.scroll_pending = 0;
}

void console_clear()
{
  console_home();
  console.cls_pending = 1;
}


void console_setfont (const uint8 *font) {
  font_width  = font[0];
  font_height = font[1];
  fontdata    = font + 2;
  font_lines  = console.dimensions.h / font_height;
}

const uint8* console_currentfont (void)
{
  return fontdata - 2;
}

static void console_blitchar(int x, int y, char ch) {
  int r,c;

  if ((y >= 0) && ((y + font_height) <= console.dimensions.h)) {
    int ofs = y * console.dimensions.w + x;
    for(r=0;r<font_height;r++) {
      for(c=0;c<font_width;c++) {
        if( (uint8)fontdata[(uint8)ch * font_height + r] & (1<<(8-c)) ) {  /* Pixel set */
          console.fb[ofs+c] = console.fgcolor;
        } else { /* Pixel clear */
          if( !console.transparent )
            console.fb[ofs+c] = console.bgcolor;
        }
      }
      ofs += console.dimensions.w;
    }
  }

  console_printcount += 1;
}

static void console_linefeed () {
  console.cursor.x = 0;
  console.cursor.y++;

  /* Check if we need to scroll the display up */
  if(console.cursor.y >= font_lines ) {
    if (console.scrollMode) {
      console.scroll_pending = 1; // delay scroll until we actually write text to a new line
    } else {
      // reset cursor to top of screen
      console.cursor.y = 0;
      console.cls_pending = 1; // we must delay fb_cls or we'd never see the just printed line!
    }
  }
  if (!console_suppress_fbupdate_cnt) {
    fb_update(console.fb);
    #ifdef MSGDELAY // actually, such a delay can now be achieved using the debug option in the config file
      unsigned int start = inl (0x60005010);
      while (inl (0x60005010) < start + MSGDELAY) {}
    #endif
  }
}

void console_putchar(char ch) {

again:
  if (console.cls_pending) {
    // clear screen
    fb_cls (console.fb, console.bgcolor);
    console.cls_pending = 0;
  } else if (console.scroll_pending) {
    // scroll
    console.scroll_pending = 0;
    int i;
    mlc_memcpy(console.fb,
               console.fb+(console.dimensions.w*font_height),
               (console.dimensions.w*console.dimensions.h*2) - 
               (console.dimensions.w*font_height*2) );
    for (i = console.dimensions.w*(console.dimensions.h-font_height); i < console.dimensions.w*console.dimensions.h; i++) {
        console.fb[i] = console.bgcolor;
    }
    console.cursor.y--;
  }
  
  if( console.cursor.x >= (console.dimensions.w / font_width) ) {
    console_linefeed ();
    goto again;
  }

  if(ch == '\n') {
    console_linefeed ();
  } else if(ch == '\r') {
    console.cursor.x = 0;
  } else {
    uint16 x,y;
    x = console.cursor.x * font_width;
    y = console.cursor.y * font_height;
    console_blitchar(x,y,ch);
    console.cursor.x += 1;
  }
}

void console_puts(volatile char *str) {
  while(*str != 0) {
    console_putchar(*str);
    str++;
  }
}

void console_putsXY(int x,int y,volatile char *str) {
  while(*str != 0) {
    console_blitchar(x,y,*str);
    x += font_width;
    str++;
  }
}

void console_init(uint16 *fb)
{
  console.ipod = ipod_get_hwinfo();

  console.cursor.x = 0;
  console.cursor.y = 0;
  console.dimensions.w = console.ipod->lcd_width;
  console.dimensions.h = console.ipod->lcd_height;

  console_setfont ( (console.dimensions.w < 300) ? font_medium : font_large );

  console.fgcolor     = WHITE;
  console.bgcolor     = BLACK;
  console.transparent = 0;
  console.scrollMode  = 1;
  console.cls_pending = 1;
  console.scroll_pending = 0;

  console.fb = fb;
}
