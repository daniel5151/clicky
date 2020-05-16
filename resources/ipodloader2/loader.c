#include "bootloader.h"

#include "ata2.h"
#include "fb.h"
#include "console.h"
#include "keypad.h"
#include "minilibc.h"
#include "ipodhw.h"
#include "vfs.h"
#include "menu.h"
#include "config.h"
#include "interrupts.h"

#define LOADERNAME "iPL Loader 2.6 " VERSION // "d" stands for development version, "b" for beta version

static uint16 *framebuffer;
static int orig_contrast;

static void shutdown_loader (void)
{
  keypad_exit ();
  ata_exit ();
  exit_irqs ();
}

static void standby ()
{
  shutdown_loader ();
  ipod_set_backlight (0);
  fb_cls (framebuffer, ipod_get_hwinfo()->lcd_is_grayscale?BLACK:WHITE);
  fb_update(framebuffer);
  mlc_delay_ms (1000);
  pcf_standby_mode ();
}

static void spindown_disk ()
{
  config_t *conf = config_get();
  if (conf->ata_standby_code >= 0) {
    ata_standby (conf->ata_standby_code);	// stop the disk (spin it down)
  }
}

static void test_contrast (config_t *conf)
{
  int linemode = 0;
  int contrast = orig_contrast;
  int redraw = 1;
  int kbdstate = 0, lastkbd = 0;
  int backlight = conf->backlight;
  uint16 linecolor = 0xffff;
  ipod_t *ipod = ipod_get_hwinfo();

  menu_init();
  console_setcolor(WHITE, BLACK, 0);

  while (1) {
    int key;

    if (redraw) {
      redraw = 0;
      lcd_set_contrast (contrast);
      console_clear();
      console_suppress_fbupdate (1); // suppresses fb_update calls for now
      mlc_printf ("Contrast test screen\n");
      mlc_printf ("Key state: %x\n", kbdstate);
      mlc_printf ("<< >>: contrast %d\n", (int)lcd_curr_contrast());
      mlc_printf ("Menu: linemode %d\n", linemode);
      mlc_printf ("Play: backlight %d\n", backlight);
      mlc_printf ("Select: exit\n");
      linecolor = fb_rgb(linemode << 6,linemode << 6,linemode << 6);
      {
        int w = ipod->lcd_width;
        menu_hline (framebuffer, 0, w-1, 78, linecolor);
        menu_drawrect (framebuffer, 111, 82, w-1, 95, linecolor);
        menu_drawrect (framebuffer, 0, 96, 110, 109, linecolor);
      }
      console_suppress_fbupdate (-1); // calls fb_update now
    }

    key = keypad_getkey();
    redraw = 1;
    if (key == IPOD_KEY_REW) {
      contrast -= 1;
    } else if (key == IPOD_KEY_FWD) {
      contrast += 1;
    } else if (key == IPOD_KEY_MENU) {
      if (++linemode > 3) linemode = 0;
    } else if (key == IPOD_KEY_PLAY) {
      backlight = !backlight;
      ipod_set_backlight (backlight);
      redraw = 0;
    } else if (key == IPOD_KEY_SELECT) {
      console_printcount = 0; // prevents userconfirm() from doing something
      return;
    } else {
      redraw = 0;
    }
    
    kbdstate = keypad_getstate();
    if (kbdstate != lastkbd) {
      lastkbd = kbdstate;
      redraw = 1;
    }
  } // while

}

static void test_piezo ()
{
  int redraw = 1, duration = 50, period = 30;
  do {
    int key;

    if (redraw) {
      redraw = 0;
      ipod_beep (duration, period);
      console_clear();
      console_suppress_fbupdate (1); // suppresses fb_update calls for now
      mlc_printf ("Piezo test\n");
      mlc_printf ("<</>>: duration %d\n", duration);
      mlc_printf ("Mnu/Play: pitch %d\n", period);
      mlc_printf ("Select: sound\n");
      mlc_printf ("<< and >>: exit\n");
      console_suppress_fbupdate (-1); // calls fb_update now
    }

    key = keypad_getkey();
    if (key) {
      redraw = 1;
      int step = period / 10;
      if (!step) step = 1;
      if (key == IPOD_KEY_REW) {
        if (duration > 0) duration -= 1;
      } else if (key == IPOD_KEY_FWD) {
        duration += 1;
      } else if (key == IPOD_KEY_MENU) {
        if (period > 0) period -= step;
      } else if (key == IPOD_KEY_PLAY) {
        period += step;
      }
    }
  } while (keypad_getstate() != (IPOD_KEYPAD_PREV+IPOD_KEYPAD_NEXT));
  console_printcount = 0; // prevents userconfirm() from doing something
}

static void *iram_get_end_ptr (ipod_t *ipod, int offset) 
{
    return (void *)(ipod->iram_base + ipod->iram_full_size - 0x100 + offset);
}

static void set_boot_action (ipod_t *ipod, const char *str) {
  mlc_memcpy (iram_get_end_ptr (ipod, 0x0), str, 8);
  mlc_memcpy (iram_get_end_ptr (ipod, 0x8), "hotstuff", 8);
  outl (1, (unsigned long)iram_get_end_ptr (ipod, 0x10));
  if (ipod->hw_rev >= 0x40000) {
    outl(inl(0x60006004) | 0x4, 0x60006004);
  } else {
    outl(inl(0xcf005030) | 0x4, 0xcf005030);
  }
}

static short calc_checksum2 (char* dest, int size) {
  short csum = 0;
  while (size-- > 0) {
    char b = *dest++;
    csum = ((csum << 1) & 0xffff) + ((csum<0)? 1 : 0) + b; // csum-rotation plus b
  }
  return csum;
}

static char* getArgs (char* baseAddr) {
  // fetch the args
  if (mlc_strncmp (baseAddr, "Args", 4) == 0) {
    int strlen = *(short*)(baseAddr+6);
    if (*(short*)(baseAddr+4) == calc_checksum2 (baseAddr+6, strlen+2)) {
      return baseAddr + 8;
    }
  }
  return 0;
}

static void setArgs (char* baseAddr, int size, char* args) {
  int strlen = mlc_strlen (args);
  // first, make sure the space we want to use is empty:
  int n = size;
  char* p = baseAddr;
  while (n-- > 0) {
    if (*p++) {
      mlc_printf ("Err: setArgs mem ~zero\n");
      return;
    }
  }
  // now fill it up:
  size -= 9;
  if (strlen > size) {
    mlc_printf ("Args too long by %d chars\n", strlen-size);
    strlen = size;
  }
  // offset 0: "Args", ofs 4: 2-byte checksum of strlen+string, ofs 6: 2-byte strlen, ofs 8: 0-terminated string
  mlc_memcpy (baseAddr, "Args", 4);
  *(short*)(baseAddr+6) = strlen;
  mlc_memcpy (baseAddr+8, args, strlen);
  baseAddr[8+strlen] = 0;
  *(short*)(baseAddr+4) = calc_checksum2 (baseAddr+6, strlen+2);
  if (mlc_strcmp (args, getArgs (baseAddr)) != 0) {
    mlc_printf ("Internal err: getArgs\n");
  }
}


//
// this function is to be called before the screen gets cleared so that
// the user can, in debug mode, confirm to continue by a keypress
//
static int userconfirm ()
{
  int shown = 0;
  config_t *conf = config_get();
  if (console_printcount) {
    if (conf->debug & 2) {
      keypad_flush ();
      mlc_printf ("-Press a key-\n");
      do { } while (!keypad_getkey());
      shown = 1;
    } else if (conf->debug) {
      // do this always in debug mode, not just if bit 0 is set
      mlc_delay_ms (3000); // 3s
      keypad_flush ();
      shown = 1;
    }
    console_printcount = 0;
  }
  if (shown) {
    if (conf->backlight) ipod_set_backlight (1);
  }
  return shown;
}

// -----------------------------
//   image file type detection
// -----------------------------

static char* rockboxIDs[] = { "ipco","nano","ipvd","ip3g","ip4g","mini", "mn2g", 0 };

static int is_rockbox_img (char *firstblock) {
  long **ids = (long **) rockboxIDs;
  long cmpval = *(long*)(firstblock+4);
  while (*ids) {
    if (cmpval == **ids) {
      // we have a match - seems to be a rockbox image
      return 1;
    }
    ++ids;
  }
  return 0;
}

static int is_fw_img_hdr (char *data) {
  return (mlc_memcmp (data, "!ATA", 4) == 0) && (*(long*)&data[500] == 0);
}

int is_applefw_img (char *firstblock); // we call this in config.c
int is_applefw_img (char *firstblock) {
  // note: this only works to check the img in ram because we do not check the
  // first 0x20 bytes but those after it - the first 0x20 bytes are not valid
  // any more because the interrupt vectors are installed there.
  return (mlc_memcmp (firstblock+0x20, "portalpl", 8) == 0);
}

static int is_linux_img (char *firstblock) {
  return (mlc_memcmp (firstblock, "\xfe\x1f\x00\xea", 4) == 0);
}

uint32 calc_checksum_fw (char* dest, int size); // we call this in config.c
uint32 calc_checksum_fw (char* dest, int size) {
  // Calculate checksum the way the firmware images are doing it
  uint32 sum = 0;
  long i;
  for (i = 0; i < size; i++) {
      sum += (uint8) *dest++;
  }
  return sum;
}


// ----------------------
//    Rockbox loading
// ----------------------

static void load_rockbox(ipod_t *ipod, int fd, uint32 fsize, uint32 read, void *entry, void *firstblock) {
  uint8 header[12];
  unsigned long chksum;
  unsigned long sum;
  int i;

  // since the first block is already read to memory, we need to move it a bit around
  mlc_memcpy (header, firstblock, 8);
  header[8]=0;
  mlc_memcpy (firstblock, (char*)firstblock+8, read-8);
  fsize -= 8;
  read -= 8;

  // The checksum is always stored in big-endian
  chksum = (header[0]<<24)|(header[1]<<16)|(header[2]<<8)|header[3];

  mlc_printf("Model: %s\n",&header[4]);
  mlc_printf("Checksum: 0x%08x\n",chksum);

  // Check that we are running the correct version of Rockbox for this
  // iPod.
  switch (ipod->hw_ver) {
    case 0x6: // Color/Photo
      if (mlc_memcmp(&header[4],rockboxIDs[0],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=3;
      break;
    case 0xc: // Nano
      if (mlc_memcmp(&header[4],rockboxIDs[1],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=4;
      break;
    case 0xb: // 5g (Video)
      if (mlc_memcmp(&header[4],rockboxIDs[2],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=5;
      break;
    case 0x1:
    case 0x2:
    case 0x3: // 3g
      if (mlc_memcmp(&header[4],rockboxIDs[3],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=7;
      break;
    case 0x5: // 4g
      if (mlc_memcmp(&header[4],rockboxIDs[4],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=8;
      break;
    case 0x4: // 1st Gen mini
      if (mlc_memcmp(&header[4],rockboxIDs[5],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=9;
      break;
    case 0x7: // 2nd Gen mini
      if (mlc_memcmp(&header[4],rockboxIDs[6],4)!=0) {
        mlc_printf("Invalid model.\n");
        return;
      }
      sum=11;
      break;


    // Note: if you add more ids here, please use the rockboxIDs[] array
    // so that the auto-detection of rb files keeps working in is_rockbox_img()

    default: // Unsupported
      mlc_printf("Invalid model.\n");
      return;
  }

  userconfirm ();

  // checksum the first block
  for (i = 0; i < read; i++) {
    sum += ((uint8*)firstblock)[i];
  }

  // Read the rest of Rockbox
  while(read < fsize) {
    long n = fsize - read, i;
    uint8 *p = (uint8*)entry + read;
    if( n > (128*1024) ) {
      n = 128*1024;
    }
    vfs_read( p, n, 1, fd );
    // checksum the blocks we just read
    for (i = 0; i < n; i++) {
      sum += *p++;
    }
    read += n;

    menu_drawprogress(framebuffer,(read * 255) / fsize);
    fb_update(framebuffer);
  }
  
  console_setcolor (WHITE, BLACK, 1);
  console_home ();
  if (sum == chksum) {
    mlc_printf("Checksum OK - starting Rockbox.\n");
  } else {
    mlc_printf("Checksum error! Aborting.\n");
    return;
  }
  userconfirm();

  shutdown_loader ();                    // this turns off interrupt handling, ...
  mlc_memcpy (entry, firstblock, 512-8); //  ... which allows us to install the first block in the right place

  // Store the IPOD hw revision in last 4 bytes of DRAM for use by Rockbox
  // and transfer execution directly to Rockbox - we don't want to run
  // the rest of the bootloader startup code.
  if (ipod->hw_rev < 0x40000) {  // PP5002
    outl(ipod->hw_rev,0x29fffffc);
  } else {                        // PP502x

    // (Note by TT: as of Mar2006, the extra RAM in 60GB iPod is ignored by
    // rockbox, so that this still works with this address at end of 32MB RAM.
    // "linuxstb" from their dev team expressed the desire to change this
    // eventually, though):
    outl(ipod->hw_rev,0x11fffffc);
  }
}


// ------------------------------
//   generic image file loading
// ------------------------------

static void *loader_handleImage (ipod_t *ipod, char *imagename, int forceRockbox) {
  int fd, isLinux = 0;
  char *txt, *args;
  uint32 fsize, n;
  int shown, showWarning = 0;
  static char *buf512 = 0;
  void *entry = (void*)ipod->mem_base;
  config_t *conf = config_get();

  if (!buf512) buf512 = mlc_malloc(512);
  
  //mlc_printf("Addr: %08lx\n", entry);

  args = mlc_strchr (imagename, ' ');
  if (args) {
    *args++ = 0;  // the file name ends with the first blank - then come the args
    while (*args == ' ' || *args == '\t') args++; 
  }

  mlc_printf("File: %s\n", imagename);
  fd = vfs_open(imagename);
  if(fd < 0) {
    mlc_printf("Err: open failed\n");
    return 0;
  }
  
  /* Get the size of the image-file */
  vfs_seek(fd,0,VFS_SEEK_END);
  fsize = vfs_tell(fd);
  vfs_seek(fd,0,VFS_SEEK_SET);
  mlc_printf("Size: %u\n",fsize);

  // read the first block of the image and see what type it is
  //  Note: do not load the first block to *entry, because it's also mapped to address 0,
  //  where we have our interupt vectors which must not be overwritten yet.
  do {
    n = vfs_read( buf512, 1, 512, fd );
    if (n != 512) {
        mlc_printf("Err: read failed\n");
        return 0;
    }
  } while (is_fw_img_hdr (buf512)); // skip the header block we might have in an extracted apple-os.bin file


  if (is_applefw_img (buf512)) {
    // we've got the apple_os
    txt = "Apple OS";
  } else if (is_linux_img (buf512)) {
    // we've got the linux kernel
    txt = "Linux kernel";
    isLinux = 1;
  } else if (is_rockbox_img (buf512)) {
    // we've got a rockbox file
    txt = "Rockbox";
    forceRockbox = 1;
  } else if (forceRockbox) {
    // the user told us in the config to handle this as a rockbox file
    txt = "Rockbox (forced)";
  } else {
    // we've got something else - just launch it blindly
    txt = "Unknown!";
    showWarning = 1;
  }

  mlc_printf("Type: %s\n",txt);
  if (isLinux && args) {
    mlc_printf("Args: %s\n", args);
  } else {
    args = 0;
  }

  if (forceRockbox) {
    // pass this on to the rockbox loader now
    
    if (isHoldEngaged()) {
      // Rockbox resets its settings when starting with Hold engaged.
      // This case only happens when Rockbox is the menu default.
      // So, let the user unlock it here - he can then still engage Hold
      // again while the file is loaded if he really wants a settings reset.
      mlc_clear_screen();
      mlc_set_output_options (0, 0);
      mlc_printf("\nRelease HOLD to continue\n");
      ipod_set_backlight (1);
      if (conf->beep_time) ipod_beep (conf->beep_time, conf->beep_period);
      int starttime = timer_get_current();
      while (isHoldEngaged()) {
        // wait for two minutes, then put iPod to sleep
        if (timer_passed (starttime, 2*TIMER_MINUTE)) {
          standby ();
        }
      }
    }
    
    load_rockbox (ipod, fd, fsize, 512, entry, buf512);
    return entry;
  }

  if (args) {
    // pass a string to linux by storing it at offset 0x80-0xFF
    // note: if you change this mem area, then also update the "getLoader2Args" tool
    //       as well as the cmdline support in the kernel (ipod_fixup inside arch.c)!
    setArgs (buf512+0x80, 0x180, args); // the space between 0x20 and 0x200 should always be free in the kernel
  }

  shown = userconfirm ();
  if (showWarning && !shown) {
    mlc_show_critical_error ();
  }

  uint32 read = 512;
  while (read < fsize) {
    if( (fsize-read) > (128*1024) ) { /* More than 128K to read */
      vfs_read( (void*)((uint8*)entry + read), 128*1024, 1, fd );
      read += 128 * 1024;
    } else { /* Last part of the file */
      vfs_read( (void*)((uint8*)entry + read), fsize-read, 1, fd );
      read = fsize;
    }

    menu_drawprogress(framebuffer,(read * 255) / fsize);
    fb_update(framebuffer);
  }

  console_setcolor (WHITE, BLACK, 1);
  console_home (); // if we get here, then there were no errors, so we can safely reset the output cursor
  mlc_printf("Load succeeded\n");

  shutdown_loader ();              // this turns off interrupt handling ...
  mlc_memcpy (entry, buf512, 512); //  ... which allows us to install the first block finally
  return entry;
}

// -----------------------
//  main entry of loader2
// -----------------------

void *loader(void) {
  int menuPos, done;
  uint32 ret;
  ipod_t *ipod;
  config_t *conf;

  ipod_init_hardware();
  ipod = ipod_get_hwinfo();
  mlc_malloc_init();

  // Delaying mlc_printf output - here's the deal (by TT 31Mar06):
  //  The goal is not to print out text if the user prefers to have a "clean" screen
  //  with just the graphics (menu and progress bar) shown. The idea is to let the
  //  user choose this "no text" option in the config file. Problem is that we can
  //  only know what's in the config file after we've read it, and that requires a
  //  lot of code run, and that code might want to print useful information in case
  //  something goes wrong.
  //  That's why there is now a "buffered" printf mode: When enabled (which it will
  //  be from startup on), mlc_printf output will be buffered and not appear on
  //  screen. Once the buffered output is turned off, the buffered text will be
  //  printed.
  //  But what if there's a "emergency", i.e. a fatal problem that prevents us to
  //  ever read the config file? Then the buffer will never be shown.
  //  For that reason, we have another 2 functions called mlc_show_critical_error()
  //  and mlc_show_fatal_error(), which, when called, will not only flush the text
  //  to screen but might to other things that are generally useful in such a case.
  //
  mlc_set_output_options (1, 0);  // this caches screen text output for now

  init_irqs (); // basic intr initialization - does not enable IRQs yet
  
  framebuffer = (uint16*)mlc_malloc( ipod->lcd_width * ipod->lcd_height * 2 );
  fb_init();
  fb_cls(framebuffer, BLACK);
  fb_update (framebuffer);

  orig_contrast = lcd_curr_contrast();
  if (ipod->lcd_is_grayscale && ipod->hw_ver >= 3) {
    // increase the contrast a little on 3G, 4G and Minis because of their crappy LCDs
    // whose contrast weakens with certain patterns (e.g. horizontal lines as they appear
    // in the menu's frame)
    lcd_set_contrast (orig_contrast + 4);
  }

  console_init(framebuffer);

  mlc_printf(LOADERNAME"\niPod: %08lx\n", ipod->hw_rev);

  keypad_init();

  // use this to test for keys held down at startup:
  uint8 startup_keys = keypad_getstate ();
  if (startup_keys) {
    mlc_printf("keys: %x\n", startup_keys);
    if (startup_keys & IPOD_KEYPAD_PREV) {
      // Rewind is held down at start
    }
  }

  ret = ata_init();
  if( ret ) {
    mlc_printf("ATAinit: %i\n",ret);
    mlc_show_fatal_error ();
  }

  ata_identify();
  vfs_init();

  config_init();
  conf = config_get();

  if (conf->debug) {
    // any non-zero debug value turns on printf console output
    // furthermore, the debug value's bits have the following meaning:
    //  0 or any other bit set: enable printf output
    //  1: confirmation: if set, wait for a keypress before text may be vanishing,
    //       otherwise just pause for 3 seconds before continuing
    //  2: slow output: make a delay after each printf()
    //  3: scrolling test (it's not clear if scrolling still crashes on some iPods; TT 31Mar06)
    mlc_printf("Debug=%d\n", conf->debug);
    mlc_set_output_options (0, conf->debug & 4);
    if (conf->backlight) ipod_set_backlight (1);
  }

  {
    int contrast;
    if (conf->contrast < 64) {
      contrast = lcd_curr_contrast() + conf->contrast;
    } else {
      contrast = conf->contrast;
    }
    lcd_set_contrast (contrast);
  }

  if (!(conf->debug & 4096)) {
    enable_irqs ();
  } else {
    mlc_printf("IRQs NOT enabled\n");
  }

  /*
   * various operations for debugging and testing parameters
   */
  if (conf->debug & 8) { // test scrolling
    int i;
    for (i = 1; i <= 15; ++i) mlc_printf ("%i\n",i);
    userconfirm ();
  }
  if (conf->debug & 16) { // test contrast, mainly for grayscale ipods
    userconfirm ();
    test_contrast (conf);
  }
  if (conf->debug & 32) { // test keypad
    userconfirm ();
    keypad_test ();
  }
  if (conf->debug & 64) { // test sound
    userconfirm ();
    test_piezo ();
  }

  menu_init();
  for(menuPos=0;menuPos<conf->items;menuPos++) {
    menu_additem( conf->image[menuPos].title );
  }

  keypad_flush (); // discard buttons that were already pressed at start

  /*---------------------------------------
   * This is the "event loop" for the menu
   */
redoMenu:
  menuPos = conf->def - 1;
  if (menuPos < 0) menuPos = 0;
  done    = 0;

  userconfirm ();
  mlc_clear_screen ();

  int startTime = timer_get_current ();
  char needsupdate = 1;
  int last_second = 0;
  int lastHold = 0;
  int idle_starttime = -1;
  int did_beep = 0;
  int did_blacklight_off = 0;

  if (conf->beep_time) ipod_beep (conf->beep_time, conf->beep_period);

  while(!done) {

    int key, isHold = isHoldEngaged();

    while ((key = keypad_getkey()) != 0) {
      // keep looping as long as we get keys so we can catch up
      // (keys may have queued up while we were updating the menu)
      if( key == IPOD_KEY_REW || key == IPOD_KEY_MENU ) {
        if (menuPos>0) menuPos--;
      } else if( key == IPOD_KEY_FWD || key == IPOD_KEY_PLAY ) {
        if (menuPos<(conf->items-1)) menuPos++;
      } else if( key == IPOD_KEY_SELECT ) {
        done = 1;
      }
      conf->timeout = 0; // user has pressed a key -> stop auto-selection timer
      needsupdate = 1;
    }
    if (isHold != lastHold) {
      if (!isHold && lastHold) conf->timeout = 0; // user has unlocked -> stop auto-selection timer
      lastHold = isHold;
      needsupdate = 1;
    }
    
    char timeLeft[4];
    timeLeft[0] = 0;
    if (conf->timeout) {
      int t = conf->timeout - (timer_get_current() - startTime) / TIMER_SECOND;
      if (t < 0) t = 0;
      if (t != last_second) {
        last_second = t;
        needsupdate = 1;
      }
      // show two digits
      timeLeft[1] = (t % 10) + '0';
      t /= 10;
      timeLeft[0] = t ? t+'0' : ' ';
      timeLeft[2] = 0;
      if (timer_passed (startTime, conf->timeout * TIMER_SECOND)) {
        // timed out
        done = 1;
      }
    }

    if (needsupdate) {
      if (conf->beep_time) keypad_enable_wheelclicks (menuPos, conf->items-menuPos-1);
      if (conf->backlight) ipod_set_backlight (1);
      needsupdate = 0;
      menu_redraw(framebuffer, menuPos, LOADERNAME, timeLeft, isHold);
      fb_update(framebuffer);
      spindown_disk ();
      idle_starttime = timer_get_current();
      did_blacklight_off = 0;
      did_beep = 0;
    }

    if (!did_blacklight_off && timer_passed (idle_starttime, 10*TIMER_SECOND)) {
      // if nothing happened for 10 seconds, then turn off backlight to save power
      ipod_set_backlight (0);
      did_blacklight_off = 1;
    }
    if (!did_beep && timer_passed (idle_starttime, 1*TIMER_MINUTE)) {
      // if nothing happened for one minute, issue a beep as a reminder
      if (conf->beep_time) ipod_beep (conf->beep_time, conf->beep_period);
      did_beep = 1;
    }
    if (timer_passed (idle_starttime, 2*TIMER_MINUTE)) {
      // if nothing happened for two minutes, then put iPod to sleep to save power
      standby ();
    }

  }
  /*
   * End of the "event loop" for the menu
   *--------------------------------------
   */

  menu_cls(framebuffer);
  fb_update(framebuffer);
  
  int forceRockbox = conf->image[menuPos].type == CONFIG_IMAGE_ROCKBOX;
  if( conf->image[menuPos].type == CONFIG_IMAGE_BINARY || forceRockbox ) {
    ret = (int) loader_handleImage (ipod, conf->image[menuPos].path, forceRockbox);
    if (!ret) {
      // load failed
      mlc_show_critical_error();
    } else {
      if (is_applefw_img ((void*)ipod->mem_base)) {
        lcd_set_contrast (orig_contrast);
        if (!conf->debug) ipod_set_backlight (0); // this seems to be necessary so that backlight dimming works on 4G and Photo models
      }
      mlc_printf("Jmp to %x\n", ret);
      return (void*) ret;
    }
  } else if( conf->image[menuPos].type == CONFIG_IMAGE_SPECIAL ) {
    char *cmd = conf->image[menuPos].path;
    if (mlc_strcmp ("standby", cmd) == 0 || mlc_strcmp ("sleep", cmd) == 0) {
      mlc_printf("Going into standby mode\n", cmd);
      userconfirm ();
      standby ();
    } else if (mlc_strcmp ("osos", cmd) == 0 || mlc_strcmp ("ramimg", cmd) == 0) {
      shutdown_loader ();
      if (is_applefw_img ((void*)ipod->mem_base)) {
        mlc_printf("Launching Apple OS\n");
      } else {
        mlc_printf("Launching from RAM\n");
      }
      lcd_set_contrast (orig_contrast); // restore contrast in case launch fails and loader() is entered again
      if (!conf->debug) ipod_set_backlight (0); // this seems to be necessary so that backlight dimming works on 4G and Photo models
	  ret = ipod->mem_base;
      mlc_printf("Jmp to %x\n", ret);
      return (void*) ret;
    } else if (mlc_strcmp ("reboot", cmd) == 0 || mlc_strcmp ("diskmode", cmd) == 0) {
      mlc_printf("Boot command:\n%s\n", cmd);
      userconfirm ();
      shutdown_loader ();
      lcd_set_contrast (orig_contrast); // restore contrast in case action fails and loader() is entered again
      set_boot_action (ipod, cmd);
      ipod_reboot ();
    } else {
      mlc_printf("Unknown command:\n%s\n", cmd);
      mlc_show_critical_error();
    }
  }

  goto redoMenu;
}
