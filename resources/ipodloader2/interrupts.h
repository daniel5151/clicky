/*
 * interrupts.h
 *
 * Interrupt handling for iPodLoader2
 *
 * Written 29 Apr 2006 by Thomas Tempelmann (ipod@tempel.org)
 */

#ifndef INTERRUPTS_H
#define INTERRUPTS_H

/* PP5002 */
#define PP5002_IDE_IRQ    1
#define PP5002_SER0_IRQ   4
#define PP5002_I2S_IRQ    5
#define PP5002_SER1_IRQ   7
#define PP5002_TIMER1_IRQ 11
#define PP5002_GPIO_IRQ   14
#define PP5002_DMA_OUT_IRQ  30
#define PP5002_DMA_IN_IRQ 31

#define PP5002_VALID_IRQ(x) (x==PP5002_IDE_IRQ||x==PP5002_SER0_IRQ||x==PP5002_I2S_IRQ||x==PP5002_SER1_IRQ||x==PP5002_TIMER1_IRQ||x==PP5002_GPIO_IRQ||x==PP5002_DMA_OUT_IRQ||x==PP5002_DMA_IN_IRQ)

/* PP5020 */
#define PP5020_TIMER1_IRQ 0
#define PP5020_TIMER2_IRQ 1
#define PP5020_I2S_IRQ    10
#define PP5020_IDE_IRQ    23
#define PP5020_GPIO_IRQ   (32+0)
#define PP5020_SER0_IRQ   (32+4)
#define PP5020_SER1_IRQ   (32+5)
#define PP5020_I2C_IRQ    (32+8)

#define PP5020_VALID_IRQ(x) (x==PP5020_TIMER1_IRQ||x==PP5020_I2S_IRQ||x==PP5020_GPIO_IRQ||x==PP5020_SER0_IRQ||x==PP5020_SER1_IRQ||x==PP5020_I2C_IRQ||x==PP5020_IDE_IRQ)


struct pt_regs {
  long uregs[17];
};

typedef void (*handle_irq)(int, void *, struct pt_regs *);

/**
 *  This call allocates interrupt resources and enables the
 *  interrupt line and IRQ handling. From the point this
 *  call is made your handler function may be invoked. Since
 *  your handler function must clear any interrupt the board
 *  raises, you must take care both to initialise your hardware
 *  and to set up the interrupt handler in the right order.
 */
int request_irq (unsigned int irq, handle_irq handler, char is_shared, void *dev_id);

void disable_irq (unsigned int irq);
void enable_irq (unsigned int irq);
void init_irqs (void);
void exit_irqs (void);
void enable_irqs (void);
int  irqs_enabled (); // returns boolean whether the irq system is initialized

#endif
