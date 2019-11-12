#! /usr/bin/make -f

CROSS	=arm-none-eabi-

AS	=$(CROSS)as
CC	=$(CROSS)gcc
OBJDUMP =$(CROSS)objdump
CFLAGS	=-Wall -O0 -ffreestanding -fomit-frame-pointer -march=armv4t -g
LDFLAGS =-Wl,-Tarm_elf_40.x -nostartfiles
OBJCOPY	=$(CROSS)objcopy 
HOSTCC	=gcc
HOSTCFLAGS =-Wall -O2

default: make_fw loader.bin

%.s: %.c
	$(CC) $(CFLAGS) -S $<

%.o: %.S %.c

loader: startup.o loader.o tools.o tux.o happymac.o
	$(CC) $(LDFLAGS) -o $@ startup.o loader.o tools.o tux.o happymac.o

loader.objdump: loader
	$(OBJDUMP) -ld --prefix-addresses loader > loader.objdump

loader.bin: loader loader.objdump
	$(OBJCOPY) -O binary loader loader.bin

make_fw: make_fw.c
	$(HOSTCC) $(HOSTCFLAGS) -o make_fw make_fw.c

bin_dist: make_fw loader.bin
	mkdir ipodloader-$(VER)
	cp README make_fw loader.bin ipodloader-$(VER)
	tar zcf ../ipodloader-$(VER).tar.gz ipodloader-$(VER)
	rm -rf ipodloader-$(VER)

src_dist:
	mkdir ipodloader-src-$(VER)
	cp README Makefile arm_elf_40.x *.[sch]  ipodloader-src-$(VER)
	tar zcf ../ipodloader-src-$(VER).tar.gz ipodloader-src-$(VER)
	rm -rf ipodloader-src-$(VER)

.PHONY: clean
clean:
	rm -f make_fw loader.bin loader startup.o loader.o make_fw.o \
		tools.o tux.o happymac.o

# vim:ts=8:sts=8:sw=8:
