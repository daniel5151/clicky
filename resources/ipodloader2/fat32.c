/*
 * Notes:
 *
 *  Tempel 13Nov06
 *    It seems that certain valid file names can't be found by the open function. E.g, the name "apple_os.bin"
 *    can't be opened, but "apple-os.bin" can. This problem was already present in v2.4, it seems, so it was
 *    not introduced by the LFN changes in 2.5.
 */

#include "bootloader.h"
#include "ata2.h"
#include "vfs.h"
#include "fat32.h"
#include "minilibc.h"

#define MAX_HANDLES 10

static filesystem myfs;

typedef struct {
  uint32 offset;

  uint32 sectors_per_fat;
  uint32 root_dir_first_cluster;
  uint32 data_area_offset;
  uint32 bytes_per_cluster;

  uint16 bytes_per_sector;
  uint16 blks_per_sector;   // "blk" means 512 byte
  uint16 blks_per_cluster;
  uint16 number_of_reserved_sectors;
  uint16 sectors_per_cluster;
  uint16 entries_in_rootdir;
  uint16 entries_per_sector;
  uint8  number_of_fats;
  uint8  bits_per_fat_entry;

  fat32_file *filehandles[MAX_HANDLES];
  uint32      numHandles;
} fat_t;

static fat_t fat;

static uint8 *clusterBuffer = NULL;

// this manages a simple block cache, mainly for the FAT (fat32_findnextcluster):
static uint8 *gFATSectorBuf = 0;
static uint32 gSecNumInFATBuf = -1;
static void readToSectorBuf (uint32 sector)
{
  if (gSecNumInFATBuf != sector) {
    ata_readblocks (gFATSectorBuf, sector * fat.blks_per_sector, fat.blks_per_sector);
    gSecNumInFATBuf = sector;
  }
}

static uint32 getLE32 (uint8* p) {
  return p[0] | (p[1] << 8) | (p[2] << 16) | (p[3] << 24);
}

static uint16 getLE16 (uint8* p) {
  return p[0] | (p[1] << 8);
}

/*
 * This routine sucks, and would benefit greatly from having
 * the FAT cached in RAM.. But.. The FAT for a 4GB Nano comes
 * out to about 4MB, so that wouldn't be very feasible for
 * larger capacity drives..
 */
static uint32 fat32_findnextcluster(uint32 prev)
{
  uint32 sector, offset, ret = 0;

  // this calculates the FAT block number
  offset = (fat.offset*512 + fat.number_of_reserved_sectors*fat.bytes_per_sector) + prev * (fat.bits_per_fat_entry/8);
  sector = offset / fat.bytes_per_sector;
  offset = offset % fat.bytes_per_sector;

  readToSectorBuf (sector);

  if (fat.bits_per_fat_entry == 32) {
    ret = getLE32 (gFATSectorBuf+offset) & 0x0FFFFFFF;
    if (ret < 2 || ret >= 0x0FFFFFF0) ret = 0;
  } else if (fat.bits_per_fat_entry == 16) {
    ret = getLE16 (gFATSectorBuf+offset);
    if (ret < 2 || ret >= 0xFFF0) ret = 0;
  }

  return ret;
}

static uint32 calc_lba (uint32 start, int isRootDir)
{
  uint32 lba;
  lba  = fat.number_of_reserved_sectors + (fat.number_of_fats * fat.sectors_per_fat);
  lba += (start - 2) * fat.sectors_per_cluster + (isRootDir?0:fat.data_area_offset);
  lba = fat.offset + (lba * fat.blks_per_sector);
  //mlc_printf("LBA %ld - %ld\n", start, lba);
  return lba;
}

static uint8 lfn_checksum (const unsigned char *entryName)
// entryName must be the name filled with spaces and without the "."
// example: "FAT32   C  "
{
  uint8 sum = 0;
  for (int i = 11; i > 0; --i) {
    sum = ((sum & 1) ? 0x80 : 0) + (sum >> 1) + *entryName++;
  }
  return sum;
}

typedef struct {
  uint16 isRoot;
  uint16 entryIdx;
  uint32 cluster;
  uint8* buffer;
} dir_state;

static void* getNextRawEntry (dir_state *state)
{
  if (!state->buffer) {
    state->buffer = clusterBuffer;
  }
  uint16 idx = (state->entryIdx)++;
  if (idx % fat.entries_per_sector != 0) {
    return &state->buffer[(idx % fat.entries_per_sector) << 5];
  } else {
    // we're starting a new sector
    uint32 cluster_lba;
    uint16 sectorIdx = idx / fat.entries_per_sector; // there are 16 entries in a 512-byte sector
    if (state->isRoot && fat.entries_in_rootdir > 0) {
      // it's a FAT16 root dir - all its sectors are in succession
      if (idx >= fat.entries_in_rootdir) {
        // end of root dir
        return 0;
      }
    } else {
      sectorIdx = sectorIdx % fat.sectors_per_cluster;
      if (sectorIdx == 0 && idx > 0) {
        // next cluster
        state->cluster = fat32_findnextcluster (state->cluster);
        if (state->cluster <= 0) {
          // no more clusters -> end of dir
          return 0;
        }
      } else {
        // next sector in same cluster
      }
    }
    cluster_lba = calc_lba (state->cluster, state->isRoot);
    ata_readblocks( state->buffer, cluster_lba + sectorIdx * fat.blks_per_sector, fat.blks_per_sector );
    return &state->buffer[0];
  }
}

static void trimr (char *s) {
  int pos = mlc_strlen(s);
  while (pos > 0 && s[pos-1] == ' ') --pos;
  s[pos] = 0;
}

static void ucs2cpy (char *dest, uint8 *ucs2src, int chars) {
  // this code simply ignores any non-ASCII unicode names, since the rest of loader2 doesn't support more of unicode either
  while (chars--) {
    *dest++ = *ucs2src;
    ucs2src += 2;
  }
}

static int getNextCompleteEntry (dir_state *dstate, char* *shortnameOut, char* *longnameOut, uint32 *cluster, uint32 *flength, uint8 *ftype)
{
  typedef struct {
    uint8    seq;    /* sequence number for slot, ored with 0x40 for last slot (= first in dir) */
    uint8    name0_4[10]; /* first 5 characters in name */
    uint8    attr;    /* attribute byte, = 0x0F */
    uint8    reserved;  /* always 0 */
    uint8    alias_checksum;  /* checksum for 8.3 alias, see lfn_checksum() */
    uint8    name5_10[12];  /* 6 more characters in name */
    uint16   start;   /* always 0 */
    uint8    name11_12[4];  /* last 2 characters in name */
  } long_dir_slot;

  static char shortname[14], longname[132];
  uint8 *entry, chksum = 0, namegood = 0;

  while ( (entry = getNextRawEntry (dstate)) != 0 ) {
    if (entry[0] == 0) {
      return 0; // end of dir
    } else if ( entry[0x0B] == 0x0F ) {
      // a long name entry
      long_dir_slot *slot = (long_dir_slot*)entry;
      int n = 13 * (slot->seq & 0x3F); // sequence number specifies offset in long name
      if (n >= 13 && n < (sizeof(longname)) && !(slot->seq & 0x80)) {
        char *ln = longname + n - 13;
        ucs2cpy (&ln[0], slot->name0_4, 5);
        ucs2cpy (&ln[5], slot->name5_10, 6);
        ucs2cpy (&ln[11], slot->name11_12, 2);
        if (slot->seq & 0x40) {
          ln[13] = 0;
          chksum = slot->alias_checksum;
          namegood = 1;
        }
      } else {
        namegood = 0;
      }
    } else {
      // A "normal" entry
      if ( entry[0] == 0xE5 ) {
        // deleted entry - continue with loop
      } else {
        *ftype = entry[0x0B];
        if (!namegood || chksum != lfn_checksum (&entry[0])) {
          // previously collected name does not belong to this entry
          longname[0] = 0;
        }
        uint32 cl = getLE16(entry+0x1A);
        if (fat.bits_per_fat_entry == 32) {
          cl |= getLE16(entry+0x14) << 16;
        }
        *cluster = cl;
        *flength = getLE32(entry+0x1C);
        if (*ftype & 8) {
          // volume label - no "." in name
          mlc_strlcpy (shortname, (char*)&entry[0], 12);
        } else {
          mlc_strlcpy (shortname, (char*)&entry[0], 9);
          trimr (shortname);
          char ext[4];
          mlc_strlcpy (ext, (char*)&entry[8], 4);
          trimr (ext);
          if (ext[0]) {
            mlc_strlcat (shortname, ".", 12);
            mlc_strlcat (shortname, ext, 12);
          }
        }
        trimr (shortname);
        *shortnameOut = shortname;
        *longnameOut = longname;
        return 1;
      }
    }
  }
  return 0; // end of dir
}

static fat32_file *fat32_findfile(uint32 startCluster, int isRoot, char *fname)
{
  uint32 flength, cluster;
  uint8  ftype;
  char *shortname, *longname;
  dir_state dstate = {isRoot, 0, startCluster, 0};
  char *next = mlc_strchr( fname, '/' );

  while ( getNextCompleteEntry (&dstate, &shortname, &longname, &cluster, &flength, &ftype) ) {
    if (*shortname == 0) {
      // deleted entry
    } else if ( (ftype & 0x1F) == 0 ) {
      // A file
      if ( mlc_strcasecmp( shortname, fname ) == 0 || mlc_strcasecmp( longname, fname ) == 0 ) {
        fat32_file *fileptr;
        fileptr = (fat32_file*)mlc_malloc( sizeof(fat32_file) );
        fileptr->cluster  = cluster;
        fileptr->opened   = 1;
        fileptr->position = 0;
        fileptr->length   = flength;
        return fileptr;
      }
    } else if ( ftype & 0x10 ) {
      // A directory
      int len = next-fname;
      if( next && (mlc_strncasecmp( shortname, fname, len ) == 0 || mlc_strncasecmp( longname, fname, len ) == 0) ) {
        return fat32_findfile( cluster, 0, next+1 );
      }
    }
  }
  return 0; // end of dir
}

static int fat32_open(void *fsdata,char *fname) {
  fat_t      *fs;
  fat32_file *file;

  fs = (fat_t*)fsdata;

  file = fat32_findfile(fs->root_dir_first_cluster,1,fname);

  if(file==NULL) {
    //mlc_printf("%s not found\n", fname);
    return(-1);
  }

  if(file != NULL) {
    if( fs->numHandles < MAX_HANDLES ) {
      fs->filehandles[fs->numHandles++] = file;
    } else return(-1);
  }

  return(fs->numHandles-1);
}

static void fat32_close (void *fsdata, int fd)
{
  fat_t *fs = (fat_t*)fsdata;
  if (fd == fs->numHandles-1) {
    --fs->numHandles;
  }
}

static size_t fat32_read(void *fsdata,void *ptr,size_t size,size_t nmemb,int fd) {
  uint32 read,toRead,lba,clusterNum,cluster,i;
  uint32 offsetInCluster, toReadInCluster;
  fat_t *fs;

  fs = (fat_t*)fsdata;

  read   = 0;
  toRead = size*nmemb;
  if( toRead > (fs->filehandles[fd]->length + fs->filehandles[fd]->position) ) {
    toRead = fs->filehandles[fd]->length + fs->filehandles[fd]->position;
  }

  /*
   * FFWD to the cluster we're positioned at
   * Could get a huge speedup if we cache this for each file
   * (Hmm.. With the addition of the sector-cache, this isn't as big of an issue, but it's still an issue though)
   */
  clusterNum = fs->filehandles[fd]->position / fs->bytes_per_cluster;
  cluster = fs->filehandles[fd]->cluster;

  for(i=0;i<clusterNum;i++) {
    cluster = fat32_findnextcluster( cluster );
  }
  
  offsetInCluster = fs->filehandles[fd]->position % fs->bytes_per_cluster;

  /* Calculate LBA for the cluster */
  lba = calc_lba (cluster, 0);
  toReadInCluster = fs->bytes_per_cluster - offsetInCluster;
  ata_readblocks( clusterBuffer, lba, ((toReadInCluster+fs->bytes_per_sector-1) / fs->bytes_per_sector) * fs->blks_per_sector );

  if( toReadInCluster > toRead ) toReadInCluster = toRead; 

  mlc_memcpy( (uint8*)ptr + read, clusterBuffer + offsetInCluster, toReadInCluster );

  read += toReadInCluster;

  /* Loops through all complete clusters */
  while(read < ((toRead / fs->bytes_per_cluster)*fs->bytes_per_cluster) ) {
    cluster = fat32_findnextcluster( cluster );
    lba = calc_lba (cluster, 0);
    ata_readblocks( clusterBuffer, lba, fs->blks_per_cluster );

    mlc_memcpy( (uint8*)ptr + read, clusterBuffer, fs->bytes_per_cluster );

    read += fs->bytes_per_cluster;
  }

  /* And the final bytes in the last cluster of the file */
  if( read < toRead ) {
    cluster = fat32_findnextcluster( cluster );
    lba = calc_lba (cluster, 0);
    ata_readblocks( clusterBuffer, lba, fs->blks_per_cluster );
    
    mlc_memcpy( (uint8*)ptr + read, clusterBuffer,toRead - read );

    read = toRead;
  }

  fs->filehandles[fd]->position += toRead;

  return(read / size);
}

static long fat32_tell(void *fsdata,int fd) {
  fat_t *fs;

  fs = (fat_t*)fsdata;

  return( fs->filehandles[fd]->position );
}

static int fat32_seek(void *fsdata,int fd,long offset,int whence) {
  fat_t *fs;

  fs = (fat_t*)fsdata;
  
  switch(whence) {
  case VFS_SEEK_CUR:
    offset += fs->filehandles[fd]->position;
    break;
  case VFS_SEEK_SET:
    break;
  case VFS_SEEK_END:
    offset += fs->filehandles[fd]->length;
    break;
  default:
    return -2;
  }

  if( offset < 0 || offset > fs->filehandles[fd]->length ) {
    return -1;
  }

  fs->filehandles[fd]->position = offset;
  return 0;
}


void fat32_newfs(uint8 part,uint32 offset) {

  // read the MBR (BPB) into memory
  uint8* bpb = (uint8*)mlc_malloc(512);
  ata_readblocks (bpb, offset, 1);

  /* Verify that this is a FAT32 partition */
  if( getLE16(bpb+510) != 0xAA55 ) {
    mlc_printf("Not valid FAT superblock\n");
    mlc_show_critical_error();
    return;
  }

  mlc_memset (&fat, 0, sizeof(fat));
  fat.offset = offset;
  fat.bytes_per_sector           = getLE16(bpb+11);
  fat.sectors_per_cluster        = bpb[0xD];
  fat.number_of_reserved_sectors = getLE16(bpb+14);
  fat.number_of_fats             = bpb[0x10];
  if (mlc_strncmp ("FAT16   ", (char*)&bpb[54], 8) == 0) {
    // FAT16 partition
    fat.sectors_per_fat            = getLE16(bpb+22);
    fat.root_dir_first_cluster     = 2;
    fat.entries_in_rootdir         = getLE16(bpb+17);
    fat.data_area_offset           = (fat.entries_in_rootdir * 32 + fat.bytes_per_sector-1) / fat.bytes_per_sector; // root directory size
    fat.bits_per_fat_entry         = 16;
  } else if (mlc_strncmp ("FAT32   ", (char*)&bpb[82], 8) == 0) {
    // FAT32 partition
    fat.sectors_per_fat            = getLE32(bpb+0x24);
    fat.root_dir_first_cluster     = getLE32(bpb+0x2C);
    fat.bits_per_fat_entry         = 32;
  } else {
    mlc_printf("Neither FAT16 nor FAT32\n");
    mlc_show_critical_error();
    return;
  }
  
  
  fat.bytes_per_cluster = fat.bytes_per_sector * fat.sectors_per_cluster;
  fat.entries_per_sector = fat.bytes_per_sector / 32;
  fat.blks_per_sector = fat.bytes_per_sector / 512;
  fat.blks_per_cluster = fat.bytes_per_cluster / 512;

  if (fat.bytes_per_sector == 512) {
    gFATSectorBuf = bpb;
  } else {
    gFATSectorBuf = (uint8*)mlc_malloc(fat.bytes_per_sector);
  }

  if( clusterBuffer == NULL ) {
    clusterBuffer = (uint8*)mlc_malloc( fat.bytes_per_cluster );
  }

  myfs.open    = fat32_open;
  myfs.close   = fat32_close;
  myfs.tell    = fat32_tell;
  myfs.seek    = fat32_seek;
  myfs.read    = fat32_read;
  myfs.getinfo = 0;
  myfs.fsdata  = (void*)&fat;
  myfs.partnum = part;
  myfs.type    = FAT32;

  vfs_registerfs( &myfs);
}
