/*
 * loader.c - iPodLinux loader
 * Copyright (c) 2003, Daniel Palffy (dpalffy (at) rainstorm.org)
 * Copyright (c) 2003, Bernard Leach (leachbj@bouncycastle.org)
 *  - Keyboard initialization, I/O
 * Copyright (C) 1996-2000 Russell King
 *  - inb, outb
 * Copyright (C) 1991, 1992  Linus Torvalds
 *  - memmove
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 */
 
#include "tools.h"

typedef struct _image {
    unsigned type;		/* '' */
    unsigned id;		/* */
    unsigned pad1;		/* 0000 0000 */
    unsigned devOffset;		/* byte offset of start of image code */
    unsigned len;		/* length in bytes of image */
    void    *addr;		/* load address */
    unsigned entryOffset;	/* execution start within image */
    unsigned chksum;		/* checksum for image */
    unsigned vers;		/* image version */
    unsigned loadAddr;		/* load address for image */
} image_t;

#define TBL ((char **)0x40000000)
#define MASK 0x1

extern image_t boot_table[];
extern img tux_hdr;
extern img happymac_hdr;

int ipod_ver = 0;

/* black magic */
static void 
init_keyboard(void)
{
	if (ipod_ver < 4) {
		/* 1..3g keyboard init */
		outb(~inb(0xcf000030), 0xcf000060);
		outb(inb(0xcf000040), 0xcf000070);

		outb(inb(0xcf000004) | 0x1, 0xcf000004);
		outb(inb(0xcf000014) | 0x1, 0xcf000014);
		outb(inb(0xcf000024) | 0x1, 0xcf000024);

		outb(0xff, 0xcf000050);
	} else if (ipod_ver == 4) {
		/* mini keyboard init */
                outl(inl(0x6000d000) | 0x3f, 0x6000d000);
                outl(inl(0x6000d010) & ~0x3f, 0x6000d010);
	} else if (ipod_ver >= 5) {
		/* 4g/photo keyboard init */

		/* nothing to do */
	}
}

static int 
key_pressed(void)
{
	unsigned char state;

	if (ipod_ver < 4) {
		state = inb(0xcf000030);
		if ((ipod_ver == 3) && ((state & 0x20) == 0)) return 0; /* hold on */
		if ((state & 0x08) == 0) return 1;
		if ((state & 0x10) == 0) return 2;
		if ((state & 0x04) == 0) return 3;
		if ((state & 0x01) == 0) return 4;
	} else if(ipod_ver == 4) {
		/* mini buttons */
		state = inb(0x6000d030);
		if ((state & 0x10) == 0) return 1;
		if ((state & 0x2) == 0) return 2;
		if ((state & 0x04) == 0) return 3;
		if ((state & 0x08) == 0) return 4;
	} else if(ipod_ver >= 5) {
		state = opto_keypad_read();
		if ((state & 0x4) == 0) return 1;
		if ((state & 0x10) == 0) return 2;
		if ((state & 0x8) == 0) return 3;
		if ((state & 0x2) == 0) return 4;
	}

	return 0;
}

static void
memmove16(void *dest, const void *src, unsigned count)
{
    struct bufstr {
	unsigned _buf[4];
    } *d, *s;

    if (src >= dest) {
	count = (count + 15) >> 4;
	d = (struct bufstr *) dest;
	s = (struct bufstr *) src;
	while (count--)
	    *d++ = *s++;
    } else {
	count = (count + 15) >> 4;
	d = (struct bufstr *)(dest + (count <<4));
	s = (struct bufstr *)(src + (count <<4));
	while (count--)
	    *--d = *--s;
    }
}	

void * 
loader(void)
{
    int imageno = 0;
    int padding = 0x4400;
    image_t *tblp = boot_table;
    void *entry;

    get_ipod_rev();
    if (ipod_ver > 3) padding = 0x4600;

    display_image(&tux_hdr, 0x0);

    wait_usec(300);

    init_keyboard();

    imageno = key_pressed();
    if (!tblp[imageno].type) imageno = 0;

    /* for appleOS as default, 0=happymac_hdr, 1=tux_hdr
       for linux as default,   0=tux_hdr, 1=happymac_hdr
    */
    switch (imageno) {
    case 0:
	    display_image(&happymac_hdr, 0x0);
	    break;

    case 1:
    default:
	    display_image(&tux_hdr, 0x0);
	    break;
    }

    tblp += imageno;
    entry = tblp->addr + tblp->entryOffset;
    if (imageno || ((int)tblp->addr & 0xffffff) != 0) {
	memmove16(tblp->addr, tblp->addr + tblp->devOffset - padding,
		tblp->len);
    }

    return entry;
}

