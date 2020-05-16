#ifndef _BOOTLOADER_H_
#define _BOOTLOADER_H_

typedef unsigned int   uint32;
typedef unsigned short uint16;
typedef unsigned char  uint8;
typedef   signed int    int32;
typedef   signed short  int16;
typedef   signed char   int8;

typedef unsigned long size_t;

#undef NULL
#define NULL ((void*)0x0)

#define inl(a) (*(volatile unsigned long *) (a))
#define outl(a,b) (*(volatile unsigned long *) (b) = (a))
#define inw(a) (*(volatile unsigned short *) (a))
#define outw(a,b) (*(volatile unsigned short *) (b) = (a))
#define inb(a) (*(volatile unsigned char *) (a))
#define outb(a,b) (*(volatile unsigned char *) (b) = (a))

typedef struct  {
	uint8	status;
	uint8	chs_start[3];
	uint8	type;		/* filesystem type: e.g. 0x0a for FAT32, 0x83 for ext2fs */
	uint8	chs_end[3];
	uint32	lba_offset;	
	uint32	lba_size;
} __attribute__((__packed__)) pt_entry_t;

typedef struct {
	uint8	code[ 0x018a];		/* MBR Code */
	uint8	ibm_ext_pte[36];	/* 4 9-byte primary partition table entries (some IBM stuff) */
	uint8	unused[10];		/* unused */
	uint32	disk_signature;		/* 4 byte disk signature */
	uint16	:16;			/* unused */
	pt_entry_t	partition_table[4];	/* the partition table */
	uint16	MBR_signature;		/* the MBR signature */
}  __attribute__((__packed__)) mbr_t;

typedef struct {
  uint8 unused1[56];
  uint16 ext2magic; /* ext2 magic bytes */
  uint8 unused2[198];
  uint8 fwfsmagic[4]; /* fwfs magic bytes */
  uint8 unused3[250];
  uint16 fat32magic; /* FAT32 magic bytes */
}  __attribute__((__packed__)) fs_header_t;


#endif


