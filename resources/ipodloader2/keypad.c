#include "bootloader.h"
#include "minilibc.h"
#include "fb.h"
#include "ipodhw.h"
#include "interrupts.h"
#include "console.h"
#include "keypad.h"

#define RTC inl(0x60005010)


static uint8 kbd_state = 0;
static int ipod_hw_ver;

static uint8 kbdbuf[4];
static int kbdbufelems = 0;

static void kbd_poll ();

uint8 keypad_getstate(void) {
  if (!irqs_enabled()) kbd_poll ();
  return kbd_state;
}

int isHoldEngaged (void)
{
  if (!irqs_enabled()) kbd_poll ();
  return (kbd_state & 0x20) != 0;
}

void keypad_flush(void)
{
  do {} while (keypad_getkey ());
}

int keypad_getkey(void)
  // fetch a key from the buffer
{
  int i, key = 0;
  if (!irqs_enabled()) kbd_poll ();
  if (kbdbufelems > 0) {
    key = kbdbuf[0];
    --kbdbufelems;
    for (i = 0; i < kbdbufelems; ++i) {
      kbdbuf[i] = kbdbuf[i+1];
    }
  }
  return key;
}

static void add_keypress (uint8 key)
  // add key to buffer
{
  if (key) {
    if (kbdbufelems < sizeof(kbdbuf)) {
      kbdbuf[kbdbufelems++] = key;
    }
  }
}

static uint8 kbdcode_to_key (int code)
{
  if (code & IPOD_KEYPAD_SCRR) return IPOD_KEY_FWD;
  if (code & IPOD_KEYPAD_NEXT) return IPOD_KEY_FWD;
  if (code & IPOD_KEYPAD_PLAY) return IPOD_KEY_PLAY;
  if (code & IPOD_KEYPAD_SCRL) return IPOD_KEY_REW;
  if (code & IPOD_KEYPAD_PREV) return IPOD_KEY_REW;
  if (code & IPOD_KEYPAD_MENU) return IPOD_KEY_MENU;
  if (code & IPOD_KEYPAD_ACTION) return IPOD_KEY_SELECT;
  // HOLD generates no key - use isHoldEngaged() instead
  return 0;
}


#define R_SC		IPOD_KEYPAD_SCRR
#define L_SC		IPOD_KEYPAD_SCRL

#define UP_SC		IPOD_KEYPAD_MENU
#define LEFT_SC		IPOD_KEYPAD_PREV
#define RIGHT_SC	IPOD_KEYPAD_NEXT
#define DOWN_SC		IPOD_KEYPAD_PLAY

#define HOLD_SC		IPOD_KEYPAD_HOLD
#define ACTION_SC	IPOD_KEYPAD_ACTION


static int last_code = 0;
static int do_clicks_fwd = 0;
static int do_clicks_rew = 0;

static void handle_scancode (uint8 code, uint8 down)
{
  if (down) {
    uint8 key = kbdcode_to_key (code);
    if (code == IPOD_KEYPAD_HOLD) {
      // since 1G and 2G models do not block the other keys in hardware, we'll do it here in software
      kbd_state = code; // this clears any other keys
    } else {
      kbd_state |= code;
    }
    add_keypress (key);
    if (code == R_SC || code == L_SC) {
      if (key == IPOD_KEY_FWD) {
        if (do_clicks_fwd-- > 0) ipod_beep (0, 0); // makes click sound
      } else if (key == IPOD_KEY_REW) {
        if (do_clicks_rew-- > 0) ipod_beep (0, 0); // makes click sound
      }
    }
  } else {
    kbd_state &= ~code;
  }
  last_code = code;
}

static void check_key (uint8 source, uint8 state, uint8 mask, uint8 code)
{
  if (source & mask) {
    handle_scancode (code, !(state & mask));
  }
}

static void handle_scroll_wheel(int new_scroll, int reverse)
{
  static int prev_scroll = -1;
  static int action_count;
  static signed char scroll_state[4][4] = {
    {0, 1, -1, 0},
    {-1, 0, 0, 1},
    {1, 0, 0, -1},
    {0, -1, 1, 0}
  };
  if ( prev_scroll >= 0 && new_scroll >= 0 ) {
    signed char state = scroll_state[prev_scroll][new_scroll];
    if (state) {
      uint8 key;
      if ((state > 0) == (reverse != 0)) {
        key = R_SC;
      } else {
        key = L_SC;
      }
      if (last_code != key) {
        action_count = 5; // we want that many wheel actions to register a key
        last_code = key;
      } else if (--action_count == 0) {
        handle_scancode (key, 1);
        handle_scancode (key, 0);
        last_code = 0; // start counting again
      }
    }
  }
  prev_scroll = new_scroll;
}

/*
 * -----------------------------------------------------------------------
 *                        Interrupt Handling
 * -----------------------------------------------------------------------
 */

#define KEYBOARD_DEV_ID	(void *)0x4b455942  // need to pass something because we use a shared irq

static uint8 was_hold = 0;

static int last_source, last_state;

static void process_keys_5002 (uint8 source)
{
  uint8 state;

  // get current keypad status
  state = inb(0xcf000030);
  outb(~state, 0xcf000060);

  last_source = source;
  last_state = state;
  
  if (ipod_hw_ver == 0x3) {
    if (was_hold && source == 0x40) {
      // debounce HOLD release (TT has no idea why this works, though)
      goto finish;
    }
    was_hold = 0;
  }

  if (source & 0x20) {
    if (ipod_hw_ver == 0x3) {
      // 3g hold switch is active low
      if (state & 0x20) {
        handle_scancode (HOLD_SC, 0);
        handle_scroll_wheel (-1, 0); // reset
        was_hold = 1;
      } else {
        handle_scancode (HOLD_SC, 1);
        state = 0x1f; // clear all keys
      }
    } else {
      handle_scancode (HOLD_SC, (state & 0x20));
      handle_scroll_wheel (-1, 0); // reset
    }
  }
  if (!(kbd_state & HOLD_SC)) {
    check_key (source, state, 0x01, RIGHT_SC);
    check_key (source, state, 0x02, ACTION_SC);
    check_key (source, state, 0x04, DOWN_SC);
    check_key (source, state, 0x08, LEFT_SC);
    check_key (source, state, 0x10, UP_SC);
    if (source & 0xc0) {
      handle_scroll_wheel ((state >> 6) & 3, 0);
    }
  }

finish:
  // ack any active interrupts
  outb(source, 0xcf000070);
}

static void kbd_intr_5002 (int irq, void *dev_id, struct pt_regs *regs)
{
  uint8 source;
  
  // we need some delay for g3, cause hold generates several interrupts, some of them delayed
  if (ipod_hw_ver == 0x3) mlc_delay_us(250);

  // get source of interupts
  source = inb(0xcf000040);
  if (source == 0) return;   // not for us

  process_keys_5002 (source);
}

static void opto_i2c_init(void)
{
  int i, curr_value;

  /* wait for value to settle */
  i = 1000;
  curr_value = (inl(0x7000c104) << 16) >> 24;
  while (i > 0) {
    int new_value = (inl(0x7000c104) << 16) >> 24;
    if (new_value != curr_value) {
      i = 10000;
      curr_value = new_value;
    } else {
      i--;
    }
  }

  outl(inl(0x6000d024) | 0x10, 0x6000d024); /* port B bit 4 = 1 */

  outl(inl(0x6000600c) | 0x10000, 0x6000600c);  /* dev enable */
  outl(inl(0x60006004) | 0x10000, 0x60006004);  /* dev reset */
  mlc_delay_us(5);
  outl(inl(0x60006004) & ~0x10000, 0x60006004); /* dev reset finish */

  outl(0xffffffff, 0x7000c120);
  outl(0xffffffff, 0x7000c124);
  outl(0xc00a1f00, 0x7000c100);
  outl(0x1000000, 0x7000c104);
}

static int button_mask = 0;

static void hdl_i2c_key (unsigned statusok, unsigned *new_button_mask, unsigned mask, uint8 key)
{
  if (statusok) {
    *new_button_mask |= mask;
    if (!(button_mask & mask)) {
      handle_scancode (key, 1);
    }
  } else if (button_mask & mask) {
    handle_scancode (key, 0);
  }
}

static int i2c_intr_count = 0;
static int i2c_last_status = 0;
static int i2c_reset_status = 0;
static int i2c_reset_count = 0;

static void key_i2c_interrupt(int irq, void *dev_id, struct pt_regs * regs)
{
  unsigned reg, status;

  static int wheelloc = -1;
  static int lasttouch = 0;
  
  if (lasttouch && ((RTC - lasttouch) > 500000)) {
    lasttouch = 0;
    wheelloc = -1;
  }

  mlc_delay_us(250);

  i2c_intr_count++;
  
  reg = 0x7000c104;

  if ((inl(0x7000c104) & 0x4000000) != 0) {
    reg = reg + 0x3C; /* 0x7000c140 */

    status = inl(0x7000c140);
    outl(0x0, 0x7000c140);  /* clear interrupt status? */
    i2c_last_status = status;

    uint16 touch = (status >> 16) & 0x7f;

    if ((status & 0x800000ff) == 0x8000001a) {
      int new_button_mask = 0;

      hdl_i2c_key (status & 0x0100, &new_button_mask, 0x01, ACTION_SC);
      hdl_i2c_key (status & 0x1000, &new_button_mask, 0x10, UP_SC);
      hdl_i2c_key (status & 0x0800, &new_button_mask, 0x08, DOWN_SC);
      hdl_i2c_key (status & 0x0200, &new_button_mask, 0x02, RIGHT_SC);
      hdl_i2c_key (status & 0x0400, &new_button_mask, 0x04, LEFT_SC);

      if ((status & 0x40000000) != 0) {
        // scroll wheel down

        int adjtouch = touch;
        if (touch > wheelloc) {
          if ((touch - wheelloc) > ((96 + wheelloc - touch)))
            adjtouch -= 96;
        } else {
          if ((wheelloc - touch) > ((96 + touch - wheelloc)))
            adjtouch += 96;
        }

        if (wheelloc == -1) {
          wheelloc = touch;
          lasttouch = RTC;
        } else if ((adjtouch - wheelloc) > 12) {
          wheelloc = touch;
          lasttouch = RTC;
          handle_scancode(R_SC, 1);
          handle_scancode(R_SC, 0);
        } else if ((adjtouch - wheelloc) < -12) {
          wheelloc = touch;
          lasttouch = RTC;
          handle_scancode(L_SC, 1);
          handle_scancode(L_SC, 0);
        } else if (wheelloc != touch) {
          lasttouch = RTC;
        }

      } else if (button_mask & 0x20) {
        // scroll wheel up
        wheelloc = -1;
      }

      button_mask = new_button_mask;

    } else if ((status & 0x800000FF) == 0x8000003A) {
      wheelloc = touch;
    } else {
      int v = (unsigned)status >> 4;
      if ((v == 0xfffffff) || (v == 0x5555555) || (v == 0xaaaaaaa)) {
        // this happens after a Hold switch release (status is then fffffffx, aaaaaaax, 5555555x)
        i2c_reset_status = status;
        i2c_reset_count++;
        opto_i2c_init();
      }
    }
  }

  if ((inl(reg) & 0x8000000) != 0) {
    outl(0xffffffff, 0x7000c120);
    outl(0xffffffff, 0x7000c124);
  }

  outl(inl(0x7000c104) | 0xC000000, 0x7000c104);
  outl(0x400a1f00, 0x7000c100);

  outl(inl(0x6000d024) | 0x10, 0x6000d024); /* port B bit 4 = 1 */
}

static void process_keys_502x (uint8 source, uint8 wheel_source, uint8 wheel_state)
{
  uint8 state;

  /* get current keypad & wheel status */
  state = inb(0x6000d030) & 0x3f;
  if (source) {
    last_source = source;
    last_state = state;
  }

  outb(~state, 0x6000d060);  // toggle interrupt level
  if (ipod_hw_ver == 0x4) {
    wheel_state = inb(0x6000d034) & 0x30;
    outb(~wheel_state, 0x6000d064);  // toggle interrupt level
  }

  if (source & 0x20) {
    // hold switch is active low
    int engaged = !(state & 0x20);
    handle_scancode (HOLD_SC, engaged);
    if (!engaged) {
      handle_scroll_wheel (-1, 0); // reset
    } else {
      state = 0x1f; // clear all keys
    }
  }
  if (ipod_hw_ver == 0x4) {
    check_key (source, state, 0x01, ACTION_SC);
    check_key (source, state, 0x02, UP_SC);
    check_key (source, state, 0x04, DOWN_SC);
    check_key (source, state, 0x08, RIGHT_SC);
    check_key (source, state, 0x10, LEFT_SC);
    if (wheel_source & 0x30) {
      handle_scroll_wheel ((wheel_state >> 4) & 3, 1);
    }
  }
}

static void key_mini_interrupt(int irq, void *dev_id, struct pt_regs * regs)
{
  uint8 source, wheel_source, wheel_state = 0;

  /* we need some delay for mini, cause hold generates several interrupts,
   * some of them delayed
   */
  mlc_delay_us(250);

  /* get source(s) of interupt */
  source = inb(0x6000d040) & 0x3f;
  if (ipod_hw_ver == 0x4) {
    wheel_source = inb(0x6000d044) & 0x30;
  } else {
    wheel_source = 0x0;
  }

  if (source == 0 && wheel_source == 0) {
    return;   // not for us
  }

  process_keys_502x (source, wheel_source, wheel_state);

  /* ack any active interrupts */
  if (source) {
    outb(source, 0x6000d070);
  }
  if (wheel_source) {
    outb(wheel_source, 0x6000d074);
  }
}

void keypad_test (void)
  // used to debug the hold switch behaviour
{
  console_setcolor(WHITE, BLACK, 0);
  do {
    int src, stt;
    console_clear();
    console_suppress_fbupdate (1); // suppresses fb_update calls for now
    mlc_printf ("Keypad test screen\n");
    if( ipod_hw_ver < 4 ) {
      src = inb(0xcf000040);
      stt = inb(0xcf000030);
    } else {
      src = inb(0x6000d040);
      stt = inb(0x6000d030);
    }
    mlc_printf (" source %02x (%02x)\n", src, last_source);
    mlc_printf (" state1 %02x (%02x)\n", stt, last_state);
    if( ipod_hw_ver >= 4 ) {
      mlc_printf (" i2c cnt %d\n", i2c_intr_count);
      mlc_printf (" %08x (%08x)\n", (int)inl(0x7000c140), i2c_last_status);
      mlc_printf (" rst %d (%08x)\n", i2c_reset_count, i2c_reset_status);
    }
    mlc_printf (" kbd_state %02x\n", kbd_state);
    mlc_printf ("press << and >> to exit\n");
    console_suppress_fbupdate (-1); // calls fb_update now
  } while (kbd_state != (IPOD_KEYPAD_PREV+IPOD_KEYPAD_NEXT));
  console_clear();
  mlc_printf ("release all buttons\n");
  while (kbd_state & 0x1f) {
    mlc_delay_ms(10);
  } // wait for buttons being released again
  kbdbufelems = 0;
  console_printcount = 0;
}

static void kbd_poll ()
{
  if (ipod_hw_ver < 4) {
    kbd_intr_5002 (0, 0, 0);
  } else {
    key_mini_interrupt (0, 0, 0);
    if (ipod_hw_ver > 4) {
      key_i2c_interrupt (0, 0, 0);
    }
  }
}

void keypad_enable_wheelclicks (int rew_left, int fwd_left)
{
  do_clicks_rew = rew_left;
  do_clicks_fwd = fwd_left;
}

void keypad_init(void)
{
  int err;
  
  ipod_hw_ver = ipod_get_hwinfo()->hw_ver;

  if( ipod_hw_ver < 4 ) {
  
    // 1G - 3G Keyboard init

    outb(~inb(0xcf000030), 0xcf000060);
    outb(inb(0xcf000040), 0xcf000070);
    
    if (ipod_hw_ver == 0x1) {
      outb(inb(0xcf000004) | 0x1, 0xcf000004);
      outb(inb(0xcf000014) | 0x1, 0xcf000014);
      outb(inb(0xcf000024) | 0x1, 0xcf000024);
    }
  
    if ((err = request_irq (PP5002_GPIO_IRQ, kbd_intr_5002, 1, KEYBOARD_DEV_ID)) != 0) {
      mlc_printf("ipodkb: IRQ %d failed: %d\n", PP5002_GPIO_IRQ, err);
      mlc_show_critical_error();
    }

    process_keys_5002 (0x3f); // get the current state of keys and hold switch
    
    // enable interrupts
    outb(0xff, 0xcf000050);
    
  } else if( ipod_hw_ver == 4 ) {

    // mini keyboard init

    /* buttons - enable as input */
    outl(inl(0x6000d000) | 0x3f, 0x6000d000);
    outl(inl(0x6000d010) & ~0x3f, 0x6000d010);

    /* scroll wheel- enable as input */
    outl(inl(0x6000d004) | 0x30, 0x6000d004); /* port b 4,5 */
    outl(inl(0x6000d014) & ~0x30, 0x6000d014); /* port b 4,5 */

    /* buttons - set interrupt levels */
    outl(~(inl(0x6000d030) & 0x3f), 0x6000d060);
    outl((inl(0x6000d040) & 0x3f), 0x6000d070);

    /* scroll wheel - set interrupt levels */
    outl(~(inl(0x6000d034) & 0x30), 0x6000d064);
    outl((inl(0x6000d044) & 0x30), 0x6000d074);

    if ((err = request_irq (PP5020_GPIO_IRQ, key_mini_interrupt, 1, KEYBOARD_DEV_ID)) != 0) {
      mlc_printf("ipodkb: IRQ %d failed: %d\n", PP5020_GPIO_IRQ, err);
      mlc_show_critical_error();
    }
    
    process_keys_502x (0x3f, 0, 0); // get the current state of keys and hold switch

    // enable interrupts
    outl(0x3f, 0x6000d050);
    outl(0x30, 0x6000d054);

  } else {

    // 4g, photo, mini2, nano etc.

    /* this call seems not be needed, and we learned that it causes problems on some models,
       so we happily skip this call:
      ipod_i2c_init();
    */
    
    opto_i2c_init();
  
    if ((err = request_irq (PP5020_GPIO_IRQ, key_mini_interrupt, 1, KEYBOARD_DEV_ID)) != 0) {
      mlc_printf("ipodkb: IRQ %d failed: %d\n", PP5020_GPIO_IRQ, err);
      mlc_show_critical_error();
    }

    if ((err = request_irq (PP5020_I2C_IRQ, key_i2c_interrupt, 1, KEYBOARD_DEV_ID)) != 0) {
      mlc_printf("ipodkb: IRQ %d (i2c) failed: %d\n", PP5020_GPIO_IRQ, err);
      mlc_show_critical_error();
    }

    process_keys_502x (0x3f, 0, 0); // get the current state of keys and hold switch

    // hold switch - enable as input
    outl(inl(0x6000d000) | 0x20, 0x6000d000);
    outl(inl(0x6000d010) & ~0x20, 0x6000d010);

    // hold switch - set interrupt levels
    outl(~(inl(0x6000d030) & 0x20), 0x6000d060);
    outl((inl(0x6000d040) & 0x20), 0x6000d070);

    // enable interrupts
    outl(0x20, 0x6000d050);
  }
}

void keypad_exit(void)
{
  // disable interrupts
  if( ipod_hw_ver < 4 ) {
    outb(0x00, 0xcf000050);
  } else {
    outl(0x00, 0x6000d050);
    outl(0x00, 0x6000d054);
  }
}
