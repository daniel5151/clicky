/*
 * startup.s - iPodLinux loader
 *
 * Copyright (c) 2003, Daniel Palffy (dpalffy (at) rainstorm.org)
 * Copyright (c) 2005, Bernard Leach <leachbj@bouncycastle.org>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 *
 *      Do not meddle in the affairs of Wizards, for they are subtle
 *      and quick to anger.
 *
 *      -"The Fellowship of the Ring", J.R.R Tolkien
 *
 * This code must not compile to more than 0x100 bytes without modifying
 * make_fw.c to accomodate the extra room required.
 */

	.equ	PP5002_PROC_ID,	0xc4000000
	.equ	PP5002_COP_CTRL, 0xcf004058

	.equ	PP5020_PROC_ID,	0x60000000
	.equ	PP5020_COP_CTRL, 0x60007004

.global _start
_start:
	/* get the high part of our execute address */
	ldr	r0, =0xff000000
	and	r8, pc, r0
	cmp	r8, #0x28000000		@ r8 is used later

	moveq	r0, #PP5002_PROC_ID
	movne	r0, #PP5020_PROC_ID
	ldr	r0, [r0]
	and	r0, r0, #0xff
	cmp	r0, #0x55
	beq	1f

	/* put us (co-processor) to sleep */
	cmp	r8, #0x28000000
	ldreq	r4, =PP5002_COP_CTRL
	moveq	r3, #0xca
	ldrne	r4, =PP5020_COP_CTRL
	movne	r3, #0x80000000
	str	r3, [r4]

	ldr	pc, =cop_wake_start

cop_wake_start:
	/* jump the COP to startup */
	ldr	r0, =startup_loc
	ldr	pc, [r0]

1:
	/* setup some stack */
	ldr	sp, =0x400177fc

	/* get the high part of our execute address */
	ldr	r2, =0xffffff00
	and	r4, pc, r2

	/* relocate to 0x40000000 */
	mov	r5, #0x40000000
	ldr	r6, =__data_start__
	sub	r0, r6, r5	/* lenth of text */
	add	r0, r4, r0	/* r0 points to start of text */
1:
	cmp	r5, r6
	ldrcc	r2, [r4], #4
	strcc	r2, [r5], #4
	bcc	1b

	ldr	pc, =start_loc	/* jump to the next instruction in 0x4000xxxx */

start_loc:
	ldr	r1, =__data_start__
	ldr	r3, =__bss_start__
	cmp	r0, r1
	beq	init_bss

1:
	cmp	r1, r3
	ldrcc	r2, [r0], #4
	strcc	r2, [r1], #4
	bcc	1b

init_bss:
	ldr	r1, =__bss_end__
	mov	r2, #0x0

1:
	cmp	r3, r1
	strcc	r2, [r3], #4
	bcc	1b

	/* go to the loader */
	bl	loader
	/* save the startup address for the COP */
	ldr	r1, =startup_loc
	str	r0, [r1]

	cmp	r8, #0x28000000
	bne	pp5020

	/* make sure COP is sleeping */
	ldr	r4, =0xcf004050
1:
	ldr	r3, [r4]
	ands	r3, r3, #0x4000
	beq	1b

	/* wake up COP */
	ldr	r4, =PP5002_COP_CTRL
	mov	r3, #0xce
	strh	r3, [r4]

	/* jump to start location */
	mov	pc, r0

pp5020:
	/* make sure COP is sleeping */
	ldr	r4, =PP5020_COP_CTRL
1:
	ldr	r3, [r4]
	ands	r3, r3, #0x80000000
	beq	1b

	/* wake up COP */
	@ ldr	r4, =PP5020_COP_CTRL
	mov	r3, #0x0
	str	r3, [r4]

	/* jump to start location */
	mov	pc, r0

startup_loc:
	.word	0x0

.align 8	/* starts at 0x100 */
.global boot_table
boot_table:
	/* here comes the boot table, don't move its offset */
	.space 400
