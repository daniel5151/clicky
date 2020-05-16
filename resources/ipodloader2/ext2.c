/*
 * Original code written by slowcoder as part of the yOS kernel
 * Distributed via IRC without license agreements
 * Original sourcecode lost over the years
 * Refound this code as part of "The Damn Small OS" kernel, without copyrights
 *
 * Reissued under the GNU Public License by original author
 * * James Jacobsson ( slowcoder@mac.com )  2005
 *
 */
#include "bootloader.h"
#include "ata2.h"
#include "vfs.h"
#include "ext2.h"
#include "minilibc.h"

#define EXT2_MAXBLOCKSIZE 4096

#define MAX_HANDLES 10

static filesystem myfs;

typedef struct {
  uint32       lba_offset;
  superblock_t super;
  group_t      groups[512];
  uint32       block_size;

  ext2_file *filehandle[MAX_HANDLES];
  uint32     numHandles;
} ext2_t;

ext2_t *ext2;

static void ext2_read_superblock(ext2_t *fs, uint32 offset) {
  ata_readblocks( &fs->super, offset + 2, 2 );
}

static void ext2_getblock(uint8 *buffer,uint32 block);

static void ext2_ReadDatablockFromInode(inode_t *inode, void *ptr,unsigned int num) {
  static uint32 *buff = 0;
  static uint32 *buff2 = 0;
  if (!buff) buff = mlc_malloc (EXT2_MAXBLOCKSIZE);
  if (!buff2) buff2 = mlc_malloc (EXT2_MAXBLOCKSIZE);

  if(        num < 12 ) { /* Direct blocks */
    ext2_getblock(ptr,inode->i_block[num]);
  } else if( num < (12 + (ext2->block_size/4) ) ) { /* Indirect blocks */
    ext2_getblock((uint8*)buff,inode->i_block[12]);

    ext2_getblock((uint8*)ptr,buff[num-12]);
  } else if( num < (12 + (ext2->block_size/4)*(ext2->block_size/4) ) ) { /* Bi-indirect blocks */
    uint32 block,offset;

    num -= (12 + (ext2->block_size/4));

    ext2_getblock((uint8*)buff2,inode->i_block[13]);

    block  = num / (ext2->block_size/4);
    offset = num % (ext2->block_size/4);

    ext2_getblock((uint8*)buff,buff2[block]);

    ext2_getblock(ptr,buff[offset]);
  } else {
    mlc_printf("Tri-indirects not supported");
    mlc_show_fatal_error ();
  }

		    
}

static unsigned short ext2_readdata(inode_t *inode,void *ptr,unsigned int off,unsigned int size){
  uint32 sblk,eblk,soff,eoff,read;

  static unsigned char *buff = 0;
  if (!buff) buff = mlc_malloc (EXT2_MAXBLOCKSIZE);
  
  read = 0;

  sblk = off          / (1024<<ext2->super.s_log_block_size);
  eblk = (off + size) / (1024<<ext2->super.s_log_block_size);
  soff = off          % (1024<<ext2->super.s_log_block_size);
  eoff = (off + size) % (1024<<ext2->super.s_log_block_size);

  /* Special case for reading less than a block */
  if( sblk == eblk ) {
    ext2_ReadDatablockFromInode(inode,buff,sblk);
    mlc_memcpy(ptr,buff + soff,eoff-soff);
    read += eoff-soff;
    return(read);
  }

  /* If we get here, we're reading cross block boundaries */
  while(read < size) {
    ext2_ReadDatablockFromInode(inode,buff,sblk);

    if(sblk != eblk) {
      mlc_memcpy(ptr,buff + soff,(1024<<ext2->super.s_log_block_size)-soff);
      read += (1024<<ext2->super.s_log_block_size)-soff;
      ptr   = (uint8*)ptr + ((1024<<ext2->super.s_log_block_size)-soff);
    } else {
      mlc_memcpy(ptr,buff,eoff);
      read += eoff;
      ptr   = (uint8*)ptr + eoff;
    }

    soff = 0;
    sblk++;

    /* See if we're done (Yes, it might be 0 bytes to read in the next block) */
    if(read==size)
      return(read);
  }

  return(read);
}

static uint32 ext2_finddirentry(uint8 *dirname,inode_t *inode) {
  dir_t dir;
  unsigned int diroff, dirlen;

  dirlen = mlc_strlen((char*)dirname);
  
  diroff = 0;
  while( diroff < inode->i_size ) {

    ext2_readdata(inode,&dir,diroff,sizeof(dir));

    if( dirlen == dir.name_len) {
      if( mlc_memcmp(dirname,dir.name,dirlen) == 0 ) {
        return(dir.inode);
      }
    }

    diroff += dir.rec_len;
  }

  return(0);
}


static void ext2_getblock(uint8 *buffer,uint32 block) {
  uint32 offset = (block << (1 + ext2->super.s_log_block_size)) + ext2->lba_offset;

  ata_readblocks(buffer,offset,1 << (ext2->super.s_log_block_size + 1));
}

static void ext2_getinode(inode_t *ptr,uint32 num) {
  uint32 block,off,group,group_offset;

  static uint8 *buff = 0;
  if (!buff) buff = mlc_malloc (EXT2_MAXBLOCKSIZE);

  num--;

  group = num / ext2->super.s_inodes_per_group;
  num  %= ext2->super.s_inodes_per_group;

  group_offset = (num * sizeof(inode_t));
  block = ext2->groups[group].bg_inode_table + group_offset / (1024 << ext2->super.s_log_block_size);
  off   = group_offset % (1024 << ext2->super.s_log_block_size);

  ext2_getblock(buff,block);
  mlc_memcpy(ptr,buff+off,sizeof(inode_t));
}

static int ext2_min(int x, int y) { return (x < y) ? x : y; } 

static void ext2_getblockgroup(void) {  /* gets our groups of blocks descriptor */
  static unsigned char *buff = 0;
  unsigned char *dest = (unsigned char *)ext2->groups;
  int block;
  int numgroups = ext2->super.s_inodes_count / ext2->super.s_inodes_per_group;
  int read = 0;

  block = ((ext2->super.s_first_data_block + 1) << (1 + ext2->super.s_log_block_size)) + ext2->lba_offset;

  if (!buff) buff = mlc_malloc (512);

  while(read < numgroups * sizeof(group_t))
    {
      ata_readblocks(buff, block++, 1);
      mlc_memcpy(dest + read, buff, ext2_min(512, (numgroups * sizeof(group_t)) - read));
      read += 512;
    }
}

static ext2_file *ext2_findfile(char *fname) {
  ext2_file *ret;
  uint32     inode_num,nstr;
  static uint8 *dirname = 0;
  inode_t   *retnode;
  //char      *origname = fname;

  if (!dirname) dirname = mlc_malloc (1024);
  
  ret     = mlc_malloc( sizeof(ext2_file) );
  retnode = &ret->inode;

  inode_num = 0x2; /* ROOT_INODE */
  ext2_getinode(retnode,inode_num);
  while( mlc_strlen(fname) != 0 ) {
    if( fname[0] == '/' ) fname++;

    nstr = 0;
    while( (*fname != '/') && (*fname != 0) ) {
      dirname[nstr++] = *fname++;
    }
    dirname[nstr] = 0x0;

    inode_num = ext2_finddirentry(dirname,retnode);
    if(inode_num == 0) {
        //mlc_printf ("%s not found\n", origname);
        return(NULL);
    }
    
    ext2_getinode(retnode,inode_num);
  }

  ret->inodeNum = inode_num;
  ret->length   = retnode->i_size;
  ret->opened   = 1;
  ret->position = 0;
 
  return(ret);
}

static int ext2_open(void *fsdata,char *fname) {
  ext2_t    *fs;
  ext2_file *file;

  fs = (ext2_t*)fsdata;

  file = ext2_findfile(fname);

  if( file == NULL ) {
    return(-1);
  }

  if( fs->numHandles < MAX_HANDLES ) {
    fs->filehandle[fs->numHandles++] = file;
  }

  return(fs->numHandles-1);
}

static void ext2_close (void *fsdata, int fd)
{
  ext2_t *fs = (ext2_t*)fsdata;
  if (fd == fs->numHandles-1) {
    --fs->numHandles;
  }
}

static int ext2_seek(void *fsdata,int fd,long offset,int whence) {
  ext2_t *fs;

  fs = (ext2_t*)fsdata;

  switch(whence) {
  case VFS_SEEK_CUR:
    offset += fs->filehandle[fd]->position;
    break;
  case VFS_SEEK_SET:
    break;
  case VFS_SEEK_END:
  	offset += fs->filehandle[fd]->length;
    break;
  default:
    return -2;
  }

  if( offset < 0 || offset > fs->filehandle[fd]->length ) {
    return -1;
  }

  fs->filehandle[fd]->position = offset;
  return 0;
}

static long ext2_tell(void *fsdata,int fd) {
  ext2_t *fs;

  fs = (ext2_t*)fsdata;

  return( fs->filehandle[fd]->position );
}

static size_t ext2_read(void *fsdata,void *ptr,size_t size,size_t nmemb,int fd) {
  uint32 toRead;
  ext2_t *fs;

  fs = (ext2_t*)fsdata;

  toRead = size*nmemb;
  if( toRead > (fs->filehandle[fd]->length - fs->filehandle[fd]->position) ) {
    toRead = fs->filehandle[fd]->length - fs->filehandle[fd]->position;
  }

  ext2_readdata(&fs->filehandle[fd]->inode, ptr, fs->filehandle[fd]->position ,toRead);

  fs->filehandle[fd]->position += toRead;

  return(toRead / size);
}

void ext2_newfs(uint8 part,uint32 offset) {
  ext2 = (ext2_t*)mlc_malloc( sizeof(ext2_t) );

  ext2_read_superblock(ext2,offset);

  if( ext2->super.s_magic == 0xEF53 ) {
    mlc_printf("ext2fs found\n");
  } else {
    mlc_printf("ext2fs NOT found\n");
    return;
  }

  ext2->numHandles = 0;
  ext2->lba_offset = offset;
  ext2->block_size = 1024 << ext2->super.s_log_block_size;

  ext2_getblockgroup();

  myfs.fsdata     = (void*)ext2;
  myfs.open       = ext2_open;
  myfs.close      = ext2_close;
  myfs.seek       = ext2_seek;
  myfs.tell       = ext2_tell;
  myfs.read       = ext2_read;
  myfs.getinfo    = 0;
  myfs.partnum    = part;
  myfs.type       = EXT2;

  vfs_registerfs(&myfs);
}
