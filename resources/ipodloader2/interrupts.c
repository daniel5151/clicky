/*
 * interrupts.c
 *
 * Interrupt handling for iPodLoader2
 *
 * Edited 26 Apr 2006 by Thomas Tempelmann (ipod@tempel.org)
 *
 * Most parts of this source were copied from kernel sources:
 *   irq.c - irq processing for iPod
 *
 * Parts copyright (c) 2003-2005 Bernard Leach (leachbj@bouncycastle.org)
 * Parts copyright (c) 1992 Linus Torvalds
 */

#include "bootloader.h"
#include "ipodhw.h"
#include "minilibc.h"
#include "interrupts.h"

/* some debug code to check for memory corruption:
static short calc_checksum2 (char* dest, int size) {
  short csum = 0;
  while (size-- > 0) {
    char b = *dest++;
    csum = ((csum << 1) & 0xffff) + ((csum<0)? 1 : 0) + b; // csum-rotation plus b
  }
  return csum;
}
static short ram_sums[1000];
static void do_cs (int check)
{
  int len = 32, i, size = 256;
  for (i = 1; i <= len; ++i) {
    short sum = calc_checksum2 (ipod_get_hwinfo()->mem_base+i * size, size);
    if (check) {
        if (ram_sums[i] != sum) {
          __cli();
          mlc_printf("\nchk %d: %x\n", check, i*size);
          mlc_show_critical_error();
          return;
        }
    } else {
        ram_sums[i] = sum;
    }
  }
}
*/

long cpu_is_502x; // used in "interrupt-entry.s"


#define __sti()                                      \
        ({                                                      \
                unsigned long temp;                             \
        __asm__ __volatile__(                                   \
        "mrs    %0, cpsr                @ local_irq_enable\n"   \
"       bic     %0, %0, #128\n"                                 \
"       msr     cpsr_c, %0"                                     \
        : "=r" (temp)                                           \
        :                                                       \
        : "memory");                                            \
        })

#define __cli()                                     \
        ({                                                      \
                unsigned long temp;                             \
        __asm__ __volatile__(                                   \
        "mrs    %0, cpsr                @ local_irq_disable\n"  \
"       orr     %0, %0, #128\n"                                 \
"       msr     cpsr_c, %0"                                     \
        : "=r" (temp)                                           \
        :                                                       \
        : "memory");                                            \
        })



#define NR_IRQS 64

struct irqaction {
  handle_irq handler;
  void *dev_id;
  char is_shared;
  struct irqaction *next;
};

struct irqdesc {
  unsigned char  nomask   : 1;    /* IRQ does not mask in IRQ   */
  unsigned char  enabled  : 1;    /* IRQ is currently enabled   */
  unsigned char  valid    : 1;    /* IRQ claimable        */
  //unsigned char  is_shared: 1;
  void (*mask_ack)(unsigned int irq); /* Mask and acknowledge IRQ   */
  void (*mask)(unsigned int irq);   /* Mask IRQ         */
  void (*unmask)(unsigned int irq); /* Unmask IRQ         */
  struct irqaction *action;
};

struct irqdesc irq_desc[NR_IRQS];


#define PP5002_TIMER1       0xcf001100
#define PP5002_TIMER1_ACK   0xcf001104
#define PP5002_TIMER_STATUS 0xcf001110

#define PP5020_TIMER1       0x60005000
#define PP5020_TIMER1_ACK   0x60005004
#define PP5020_TIMER2       0x60005008
#define PP5020_TIMER2_ACK   0x6000500c
#define PP5020_TIMER_STATUS 0x60005010


static void pp5002_unmask_irq(unsigned int irq)
{
  outl((1 << irq), 0xcf001024);
  outl(inl(0xcf00102c) & ~(1 << irq), 0xcf00102c);
}

static void pp5002_mask_irq(unsigned int irq)
{
  outl((1 << irq), 0xcf001028);
}

static void pp5002_mask_ack_irq(unsigned int irq)
{
  switch (irq) {
  case PP5002_IDE_IRQ:
    outl(0xff, 0xc0003020);
    outl(inl(0xc0003024) | (1<<4) | (1<<5), 0xc0003024);
    break;
  case PP5002_TIMER1_IRQ:
    inl(PP5002_TIMER1_ACK);
    break;
  }
  pp5002_mask_irq(irq);
}

static void pp5020_unmask_irq(unsigned int irq)
{
  switch (irq) {
  case PP5020_IDE_IRQ:
    outl(inl(0xc3000028) | (1<<5), 0xc3000028);
  }
  if (irq < 32) {
    outl((1 << irq), 0x60004024);
  } else {
    outl(0x40000000, 0x60004024);
    outl((1 << (irq - 32)), 0x60004124);
  }
}

static void pp5020_mask_irq(unsigned int irq)
{
  if (irq < 32) {
    outl((1 << irq), 0x60004028);
  } else {
    outl((1 << (irq - 32)), 0x60004128);
  }
}

static void pp5020_mask_ack_irq(unsigned int irq)
{
  switch (irq) {
  case PP5020_TIMER1_IRQ:
    inl(PP5020_TIMER1_ACK);
    break;
  case PP5020_IDE_IRQ:
    outl(inl(0xc3000028) & ~((1<<4) | (1<<5)), 0xc3000028);
    break;
  }
  pp5020_mask_irq(irq);
}

static void ipod_init_irq (int ipod_hw_ver)
{
  int irq;

  /* disable all interrupts */
  if (ipod_hw_ver > 0x3) {
    outl(-1, 0x60001138);
    outl(-1, 0x60001128);
    outl(-1, 0x6000111c);
    outl(-1, 0x60001038);
    outl(-1, 0x60001028);
    outl(-1, 0x6000101c);
  } else {
    outl(-1, 0xcf00101c);
    outl(-1, 0xcf001028);
    outl(-1, 0xcf001038);
  }

  /* clear all interrupts */
  for ( irq = 0; irq < NR_IRQS; irq++ ) {
    if (ipod_hw_ver > 0x3) {
      if (!PP5020_VALID_IRQ(irq)) continue;
    } else {
      if (!PP5002_VALID_IRQ(irq)) continue;
    }
    irq_desc[irq].valid     = 1;
    if (ipod_hw_ver > 0x3) {
      irq_desc[irq].mask_ack  = pp5020_mask_ack_irq;
      irq_desc[irq].mask      = pp5020_mask_irq;
      irq_desc[irq].unmask    = pp5020_unmask_irq;
    } else {
      irq_desc[irq].mask_ack  = pp5002_mask_ack_irq;
      irq_desc[irq].mask      = pp5002_mask_irq;
      irq_desc[irq].unmask    = pp5002_unmask_irq;
    }
  }
}

//static spinlock_t irq_controller_lock;

void do_IRQ (int irq /* gets currently not passed: , struct pt_regs * regs*/)
/*
 * Note: the code in "interrupt-entry.s" that calls this function
 * does leave IRQs disabled until return from this function.
 * This function gets entered with the IRQ still disabled. You
 * may enable it here.
 */
{
  struct irqdesc * desc;
  struct irqaction * action;

  if (irq >= NR_IRQS) {
    // spurious intr
    return;
  }
  desc = irq_desc + irq;
  if (desc->mask_ack) {
    desc->mask_ack(irq);
    action = desc->action;
    if (action) {
      if (desc->nomask) {
        desc->unmask(irq);
      }
      __sti(); // enable IRQs
      do {
        action->handler(irq, action->dev_id, 0);
        action = action->next;
      } while (action);
      __cli(); // disable IRQs so that unmask() can be called safely
      if (!desc->nomask && desc->enabled) {
        desc->unmask(irq);
      }
    }
  }
}

static int setup_arm_irq(int irq, struct irqaction * new)
{
  int shared = 0;
  struct irqaction *old, **p;
  struct irqdesc *desc = irq_desc + irq;
  p = &desc->action;
  if ((old = *p) != NULL) {
    if (!(old->is_shared && new->is_shared)) {
      // Can't share interrupts unless both agree to
      return -3;
    }
    do {
      p = &old->next;
      old = *p;
    } while (old);
    shared = 1;
  }
  *p = new;
  if (!shared) {
    desc->nomask = 0; // (new->flags & SA_IRQNOMASK) ? 1 : 0;
    desc->enabled = 1;
    desc->unmask(irq);
  }
  return 0;
}

static void unhandled_exception ()
{
  ipod_reboot ();
}

static long saved1[8];
static long saved2[2];

static void install_intr_handler ()
{
  /*
   * This code patches some low memory areas to install exception handlers
   * Things to consider:
   * - Address 0 is the SDRAM which is also at 0x10000000 (or 0x28000000) and which contains
   *   the "osos" image loaded by the Flash ROM loader.
   * - Code in "config.c" checks whether the loaded image in SDRAM is the Apple OS file. It
   *   does this by comparing the bytes at offset 0x20 against "portalpl". Therefore, we should
   *   not modify that memory area.
   * - Area from offset 0x80 - 0xFF is used to pass arguments to the kernel. Therefore, that
   *   area should not be used either.
   * This leaves the memory from 0x00 to 0x1F and 0x28 to 0x7F available for us.
   */
  int i;
  extern void interrupt_handler (void);
  mlc_memcpy (saved1, (void*)0, 32);
  mlc_memcpy (saved2, (void*)0x40, 8);
  *(volatile long*)0x40 = (long)unhandled_exception; // installs a pointer to the unhandled_exception() handler
  for (i = 0; i < 32; i+=4) {
    *(volatile long*)i = 0xe59ff000 + 0x40 - 8 - i; // preset all exception entries with a jump to unhandled_exception()
  }
  *(volatile long*)0x44 = (long)interrupt_handler; // installs a pointer to the irq handler
  *(volatile long*)0x18 = 0xe59ff000 + 0x44 - 8 - 0x18; // IRQ jumps thru vector at 0x44
}

static void restore_intr_handler ()
{
  if (saved1[0]) {
    mlc_memcpy ((void*)0, saved1, 32);
    mlc_memcpy ((void*)0x40, saved2, 8);
  }
}

static long memory_map_value[8];
static int memory_mapped = 0;

static void remap_memory (char enable)
{
  if (enable) {
    if (!memory_mapped) {
      for (int i = 0; i < 8; ++i) { memory_map_value[i] = ((volatile long*)0xf000f000)[i]; }
    }
    // map SDRAM from 0x10000000 or 0x28000000 to 0, Flash ROM from 0 to 0x20000000:
    //for debug: mlc_hexdump (0, 32);
    *(volatile long*)0xf000f010 = 0x3a00 | 0; // logical addr
    *(volatile long*)0xf000f014 = 0x3f84 | ipod_get_hwinfo()->mem_base; // physical addr
    *(volatile long*)0xf000f008 = 0x3a00 | 0x20000000; // logical addr
    *(volatile long*)0xf000f00c = 0x3f84 | 0; // physical addr
    memory_mapped = 1;
    //for debug: mlc_hexdump ((void*)0x20000000, 32); mlc_show_critical_error ();
  } else if (memory_mapped) {
    for (int i = 0; i < 8; ++i) { ((volatile long*)0xf000f000)[i] = memory_map_value[i]; }
    memory_mapped = 0;
  }
}

int request_irq (unsigned int irq, handle_irq handler, char is_shared, void *dev_id)
{
  struct irqaction *action;
  if (irq >= NR_IRQS || !irq_desc[irq].valid || !handler) return -1;
  action = (struct irqaction *) mlc_malloc (sizeof(struct irqaction));
  if (!action) return -2;
  action->handler = handler;
  action->is_shared = is_shared;
  action->next = NULL;
  action->dev_id = dev_id;
  return setup_arm_irq(irq, action);
}

void disable_irq (unsigned int irq)
{
  struct irqdesc *desc = irq_desc + irq;
  desc->enabled = 0;
  if (desc->mask) desc->mask(irq);
}

void enable_irq (unsigned int irq)
{
  struct irqdesc *desc = irq_desc + irq;
  desc->enabled = 1;
  if (desc->unmask) desc->unmask(irq);
}

static int intrs_enabled = 0;
static int intrs_inited = 0;

int irqs_enabled ()
{
  return intrs_enabled;
}

void init_irqs (void)
{
  int ipod_hw_ver = ipod_get_hwinfo()->hw_ver;
  cpu_is_502x = ipod_hw_ver > 3;
  mlc_memset (irq_desc, 0, sizeof (irq_desc));
  ipod_init_irq (ipod_hw_ver); // assigns interrupt masking handlers
  intrs_inited = 1;
}

void enable_irqs (void)
{
  if (!intrs_enabled) {
    remap_memory (1); // maps SDRAM to address 0
    install_intr_handler (); // assigns IRQ vector to "interrupt_handler" function
    __sti(); // enables IRQ
    intrs_enabled = 1;
  }
}

void exit_irqs (void)
{
  int i;
  __cli();
  intrs_inited = 0;
  for (i = 0; i < NR_IRQS; ++i) disable_irq (i);
  restore_intr_handler ();
  remap_memory (0);
}
