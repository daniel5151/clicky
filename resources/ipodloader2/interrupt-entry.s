/*
 * Interrupt handling for iPodLoader2
 *
 * Assembled 26 Apr 2006 by Thomas Tempelmann (ipod@tempel.org)
 *
 * Most parts of this source were copied from kernel sources:
 *    linux/arch/armnommu/kernel/entry-armv.S
 *
 *  Parts copyright (C) 1996,1997,1998 Russell King
 *  ARM700 fix by Matthew Godbolt (linux-user@willothewisp.demon.co.uk)
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 */



        .equ    NR_IRQS, 64

/* PP5020,PP5002 register definitions */
        .equ    PP5002_PROC_ID, 0xc4000000
        .equ    PP5002_COP_CTRL, 0xcf004058
        .equ    PP5020_PROC_ID, 0x60000000
        .equ    PP5020_COP_CTRL, 0x60007004
        .equ    PP5002_IDE_PRIMARY_BASE, 0xc00031e0
        .equ    PP5002_IDE_PRIMARY_CONTROL, 0xc00033f8
        .equ    PP5020_IDE_PRIMARY_BASE, 0xc30001e0
        .equ    PP5020_IDE_PRIMARY_CONTROL, 0xc30003f8

/* special locations in fast ram */
        .equ    PP_CPU_TYPE, 0x40000000

/* PP5002 */
        .equ    PP5002_IDE_IRQ, 1
        .equ    PP5002_SER0_IRQ, 4
        .equ    PP5002_I2S_IRQ, 5
        .equ    PP5002_SER1_IRQ, 7
        .equ    PP5002_TIMER1_IRQ, 11
        .equ    PP5002_GPIO_IRQ, 14
        .equ    PP5002_DMA_OUT_IRQ, 30
        .equ    PP5002_DMA_IN_IRQ, 31

        .equ    PP5002_IDE_MASK, (1 << PP5002_IDE_IRQ)
        .equ    PP5002_SER0_MASK, (1 << PP5002_SER0_IRQ)
        .equ    PP5002_I2S_MASK, (1 << PP5002_I2S_IRQ)
        .equ    PP5002_SER1_MASK, (1 << PP5002_SER1_IRQ)
        .equ    PP5002_TIMER1_MASK, (1 << PP5002_TIMER1_IRQ)
        .equ    PP5002_GPIO_MASK, (1 << PP5002_GPIO_IRQ)
        .equ    PP5002_DMA_OUT_MASK, (1 << PP5002_DMA_OUT_IRQ)

/* PP5020 */
        .equ    PP5020_TIMER1_IRQ, 0
        .equ    PP5020_TIMER2_IRQ, 1
        .equ    PP5020_I2S_IRQ, 10
        .equ    PP5020_IDE_IRQ, 23
        .equ    PP5020_GPIO_IRQ, (32+0)
        .equ    PP5020_SER0_IRQ, (32+4)
        .equ    PP5020_SER1_IRQ, (32+5)
        .equ    PP5020_I2C_IRQ, (32+8)

        .equ    PP5020_TIMER1_MASK, (1 << PP5020_TIMER1_IRQ)
        .equ    PP5020_I2S_MASK, (1 << PP5020_I2S_IRQ)
        .equ    PP5020_IDE_MASK, (1 << PP5020_IDE_IRQ)
        .equ    PP5020_GPIO_MASK, (1 << (PP5020_GPIO_IRQ-32))
        .equ    PP5020_SER0_MASK, (1 << (PP5020_SER0_IRQ-32))
        .equ    PP5020_SER1_MASK, (1 << (PP5020_SER1_IRQ-32))
        .equ    PP5020_I2C_MASK, (1 << (PP5020_I2C_IRQ-32))

/* CPU modes */
        .equ    MODE_MASK, 0x1f
        .equ    T_BIT, 0x20
        .equ    F_BIT, 0x40
        .equ    I_BIT, 0x80
        .equ    MODE_IRQ, 0x12
        .equ    MODE_SVC, 0x13
        .equ    MODE_SYS, 0x1f

                .text


                .globl  cpu_is_502x        @ defined in startup.s

                .macro  get_irqnr_and_base, irqnr, irqstat, base, tmp

                ldr     r0, =cpu_is_502x
                ldr     r0, [r0]
                cmp     r0, #0
                bne     1002f           @ branch if PP5020

                @ PP5002 code
                ldr     \base, =(0xcf001000)
                ldr     \irqstat, [\base]

                tst     \irqstat, #PP5002_DMA_OUT_MASK
                movne   \irqnr, #PP5002_DMA_OUT_IRQ
                bne     1001f

                tst     \irqstat, #PP5002_GPIO_MASK
                movne   \irqnr, #PP5002_GPIO_IRQ
                bne     1001f

                tst     \irqstat, #PP5002_IDE_MASK
                movne   \irqnr, #PP5002_IDE_IRQ
                bne     1001f

                tst     \irqstat, #PP5002_SER1_MASK
                movne   \irqnr, #PP5002_SER1_IRQ
                bne     1001f

                tst     \irqstat, #PP5002_I2S_MASK
                movne   \irqnr, #PP5002_I2S_IRQ
                bne     1001f

                tst     \irqstat, #PP5002_SER0_MASK
                movne   \irqnr, #PP5002_SER0_IRQ
                bne     1001f

                tst     \irqstat, #PP5002_TIMER1_MASK
                movne   \irqnr, #PP5002_TIMER1_IRQ
                bne     1001f

                b       1001f

1002:
                @ PP5020 code
                ldr     \base, =(0x64004000)
                ldr     \irqstat, [\base]

                tst     \irqstat, #PP5020_TIMER1_MASK
                movne   \irqnr, #PP5020_TIMER1_IRQ
                bne     1001f

                tst     \irqstat, #PP5020_IDE_MASK
                movne   \irqnr, #PP5020_IDE_IRQ
                bne     1001f

                ldr     \base, =(0x64004100)
                ldr     \irqstat, [\base]

                tst     \irqstat, #PP5020_GPIO_MASK
                movne   \irqnr, #PP5020_GPIO_IRQ
                bne     1001f

                tst     \irqstat, #PP5020_SER0_MASK
                movne   \irqnr, #PP5020_SER0_IRQ
                bne     1001f

                tst     \irqstat, #PP5020_SER1_MASK
                movne   \irqnr, #PP5020_SER1_IRQ
                bne     1001f

                tst     \irqstat, #PP5020_I2C_MASK
                movne   \irqnr, #PP5020_I2C_IRQ
                bne     1001f

1001:
                .endm

                .align  5
                
                /*
                 * here comes the IRQ entry handler
                 */
interrupt_handler:     .globl interrupt_handler

                @ save mode specific registers
                sub     lr, lr, #4                      @ lr = r14
                stmfd   sp!, {r12, lr}                  @ put r12 and r14 on stack (sp = r13)

                mrs     r12, spsr
                stmfd   sp!, {r12}                      @ put spsr on stack

/* we do not re-enable IRQs here - we let do_IRQ() handle it to simplify locking needs in there
                @ enable IRQ again
                mrs     r14, cpsr
                bic     r14, r14, #I_BIT
                msr     cpsr_c, r14
*/

                stmfd   sp!, {r0 - r11}                  @ save r0 - r11 on stack

                get_irqnr_and_base r0, r6, r5, r7
                beq     3f
                @ routine called with r0 = irq number, r1 = struct pt_regs *
                mov     r1, sp

                bl      do_IRQ

3:
                @
                @ return from interrupt
                @
                ldmfd   sp!, {r0 - r11}                 @ reload r0 - r11 from stack
                
                @ disable IRQ
                mrs     r12, cpsr
                orr     r12, r12, #I_BIT
                msr     cpsr_c, r12

                ldmfd   sp!, {r12}                      @ reload spsr from stack
                msr     spsr_cxsf, r12
                ldmfd   sp!, {r12, pc}^                 @ reload previous r12_irq, reload previous lr to pc

__irq_invalid:  swi     0

                .align  5
