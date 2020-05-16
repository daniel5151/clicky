#include "bootloader.h"
#include "ata2.h"
#include "vfs.h"
#include "fwfs.h"
#include "minilibc.h"

#define MAX_HANDLES 10
#define MAX_IMAGES 10

static filesystem myfs;

typedef struct {
  uint32 offset; /* Offset to partition in LBA blocks */
  uint32 images;

  uint32 numHandles;

  fwfs_header_t head;

  fwfs_file *filehandle;

  fwfs_image_t *image;
} fwfs_t;

fwfs_t fwfs;

static uint8 *gBlkBuf = 0;

static int fwfs_load_subimg_info(fwfs_image_t *master, int subnr, fwfs_image_t *sub) 
{
  if ((((subnr + 1) * sizeof(fwfs_image_t)) + (master->devOffset & 0x1ff) + 0x100) > 0x200) {
    mlc_printf ("Misaligned image - can't load subs\n");
    return 0;
  }
  ata_readblock (gBlkBuf, master->devOffset >> 9);
  mlc_memcpy (sub, gBlkBuf + (subnr * sizeof(fwfs_image_t)) + (master->devOffset & 0x1ff) + 0x100,
              sizeof(fwfs_image_t));
  /* The &0xc0c0c0c0==0x40404040 test makes sure all the chars are in the range
   * 0x40 to 0x7f inclusive - a really crude heuristic for "all letters", but
   * it's good enough for this.
   */
  if (sub->type != 0 && sub->type != 0xFFFFFFFF && (sub->type & 0xc0c0c0c0) == 0x40404040)
    return 1;
  return 0;
}

static int fwfs_open(void *fsdata,char *fname) {
  uint32 i;
  fwfs_t *fs;
  fwfs_image_t subimg;

#if DEBUG
  mlc_printf("Entering fwfs_open: %s\n", fname);
#endif

  fs = (fwfs_t*)fsdata;

  for(i=0;i<MAX_IMAGES;i++) {
    if( mlc_strncmp( (char*)&fs->image[i].type, fname, 4 ) == 0 ) { /* Found image */

      if( fs->numHandles < MAX_HANDLES ) {
        fwfs_file *fh = &fs->filehandle[fs->numHandles];
	fh->position  = 0;
	fh->length    = fs->image[i].len;
	fh->devOffset = fs->image[i].devOffset;
	fh->chksum    = fs->image[i].chksum;

        switch (fname[4]) {
        case '\0': /* full image - normal load */
          break;
        case '@': /* master image - aka the loader itself. don't know WHY, but oh well... */
          fh->devOffset += fs->image[i].entryOffset;
          fh->length    -= fs->image[i].entryOffset;
          break;
        case '0': case '1': case '2': case '3': case '4': /* sub-image, aka Apple or Linux */
          /* you can also load the default just by loading the whole thing, but
           * that's rather inefficient since you're loading Linux too...
           */
          if (!fwfs_load_subimg_info (fs->image + i, fname[4] - '0', &subimg)) {
            mlc_printf("Err: asked for invalid child img\n");
            return(-1);
          }
          fh->devOffset = subimg.devOffset;
          fh->length    = subimg.len;
          break;
        }

	fs->numHandles++;
	return(fs->numHandles-1);
      } else {
	mlc_printf("Err: out of handles\n");
	return(-1);
      }
    }
  }

#if DEBUG
  mlc_printf("Err: did not find the image\n");
#endif
  return(-1);
}

static void fwfs_close (void *fsdata, int fd)
{
  fwfs_t *fs = (fwfs_t*)fsdata;
  if (fd == fs->numHandles-1) {
    --fs->numHandles;
  }
}

static size_t fwfs_read(void *fsdata,void *ptr,size_t size,size_t nmemb,int fd) {
  fwfs_t *fs;
  uint32  block,off,read,toRead;

  fs = (fwfs_t*)fsdata;

  read   = 0;
  toRead = size * nmemb;
  off    = fs->filehandle[fd].devOffset + fs->filehandle[fd].position + (fs->offset * 512);
  if (fs->head.version == 3) {
    off += 512;
  }

  block  = off / 512;
  off    = off % 512;

  if( off != 0 ) { /* Need to read a partial block at first */
    ata_readblocks_uncached( gBlkBuf, block, 1 );
    mlc_memcpy( ptr, gBlkBuf + off, 512 - off );
    read += 512 - off;
    block++;
  }

  while( (read+512) <= toRead ) {
    ata_readblocks_uncached( (uint8*)ptr + read, block, 1 );

    read  += 512;
    block++;
  }
  ata_readblocks_uncached( gBlkBuf, block, 1 );
  mlc_memcpy( (uint8*)ptr+read, gBlkBuf, toRead - read );

  read += (toRead - read);

  fs->filehandle[fd].position += read;

  return(read);
}

static long fwfs_tell(void *fsdata,int fd) {
  fwfs_t *fs;

  fs = (fwfs_t*)fsdata;

  return( fs->filehandle[fd].position );
}

static int fwfs_seek(void *fsdata,int fd,long offset,int whence) {
  fwfs_t *fs;

  fs = (fwfs_t*)fsdata;
  
  switch(whence) {
  case VFS_SEEK_CUR:
    offset += fs->filehandle[fd].position;
    break;
  case VFS_SEEK_SET:
    break;
  case VFS_SEEK_END:
  	offset += fs->filehandle[fd].length;
    break;
  default:
    return -2;
  }

  if( offset < 0 || offset > fs->filehandle[fd].length ) {
    return -1;
  }

  fs->filehandle[fd].position = offset;
  return 0;
}

static int fwfs_getinfo (void *fsdata, int fd, long *out_chksum) {
  fwfs_t *fs;
  fs = (fwfs_t*)fsdata;
  if (out_chksum) *out_chksum = fs->filehandle[fd].chksum;
  return 0;
}

void fwfs_newfs(uint8 part,uint32 offset) {
  uint32 block,i;

  if (!gBlkBuf) gBlkBuf = mlc_malloc (512);

  /* Verify that this is indeed a firmware partition */
  ata_readblocks_uncached( gBlkBuf, offset,1 );
  if( mlc_strncmp((void*)((uint8*)gBlkBuf+0x100),"]ih[",4) != 0 ) {
    return;
  }

  /* copy the firmware header */
  mlc_memcpy(&fwfs.head, gBlkBuf + 0x100, sizeof(fwfs_header_t));

  //mlc_printf("\nversion = %d\n", (int)fwfs.head.version);

  if (fwfs.head.version == 1) {
    fwfs.head.bl_table = 0x4000;
  }

  block = offset + (fwfs.head.bl_table / 512);
 
  if (fwfs.head.version >= 2) {
 	block += 1;
  }

  //mlc_printf("\nblock = %d\n", (int)block);

  fwfs.filehandle = (fwfs_file*)mlc_malloc( sizeof(fwfs_file) * MAX_HANDLES );

  fwfs.image = (fwfs_image_t*)mlc_malloc(512);
  ata_readblocks_uncached( fwfs.image, block, 1 ); /* Reads the Bootloader image table */

  fwfs.images = 0;
  for(i=0;i<MAX_IMAGES;i++) {
    if( (fwfs.image[i].type != 0xFFFFFFFF) && (fwfs.image[i].type != 0x0) ) {

      fwfs.image[i].type = ((fwfs.image[i].type & 0xFF000000)>>24) | ((fwfs.image[i].type & 0x00FF0000)>>8) | 
	                   ((fwfs.image[i].type & 0x000000FF)<<24) | ((fwfs.image[i].type & 0x0000FF00)<<8);

      fwfs.images++;
    }
  }

  fwfs.filehandle = (fwfs_file *)mlc_malloc( sizeof(fwfs_file) * MAX_HANDLES );

  fwfs.offset     = offset;
  fwfs.numHandles = 0;

  myfs.open    = fwfs_open;
  myfs.close   = fwfs_close;
  myfs.tell    = fwfs_tell;
  myfs.seek    = fwfs_seek;
  myfs.read    = fwfs_read;
  myfs.getinfo = fwfs_getinfo;
  myfs.fsdata  = (void*)&fwfs;
  myfs.partnum = part;
  myfs.type    = FWFS;

  //mlc_printf("Registering..\n");

  vfs_registerfs( &myfs);
}
