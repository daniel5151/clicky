/*
 * macpartitions.c
 *
 * see header file for more infos
 *
 * Note on editor settings: indentation is done with TABs only (no blanks)
 *
 * Note: This current implementation can only read files if they are not too much fragmented.
 *       This means: If the file has more than 8 fragments (=extents), it can't be accessed
 *       entirely. If this happens, an error message will be printed to the iPod screen.
 *       Unfortunately, this also applies to the catalog file (directory), but hopefully the
 *       iPod can't be filled with so many files that this ever happens.
 */

#pragma pack (1)

extern "C" {
  #include "bootloader.h"
  #include "ata2.h"
  #include "fat32.h"
  #include "ext2.h"
  #include "fwfs.h"
  #include "vfs.h"
  #include "minilibc.h"
  #include "macpartitions.h"
}

/* Macros for swapping values */
#define OSSwapConstInt16(x) \
    ((uint16)((((uint16)(x) & 0xff00) >> 8) | \
              (((uint16)(x) & 0x00ff) << 8)))

#define OSSwapConstInt32(x) \
    ((uint32)((((uint32)(x) & 0xff000000) >> 24) | \
              (((uint32)(x) & 0x00ff0000) >>  8) | \
              (((uint32)(x) & 0x0000ff00) <<  8) | \
              (((uint32)(x) & 0x000000ff) << 24)))

#define OSSwapConstInt64(x) \
    ((uint64)((((uint64)(x) & 0xff00000000000000ULL) >> 56) | \
              (((uint64)(x) & 0x00ff000000000000ULL) >> 40) | \
              (((uint64)(x) & 0x0000ff0000000000ULL) >> 24) | \
              (((uint64)(x) & 0x000000ff00000000ULL) >>  8) | \
              (((uint64)(x) & 0x00000000ff000000ULL) <<  8) | \
              (((uint64)(x) & 0x0000000000ff0000ULL) << 24) | \
              (((uint64)(x) & 0x000000000000ff00ULL) << 40) | \
              (((uint64)(x) & 0x00000000000000ffULL) << 56)))

#if !ONPC
  // we are Little Endian
  #define LITTLE_ENDIAN_DECL(TYPE, SWAPPER) typedef TYPE TYPE##le
  #define BIG_ENDIAN_DECL(TYPE, SWAPPER) \
    class TYPE##be { public: \
      TYPE##be & operator = (TYPE arg) { this->endianSwappedVal = SWAPPER(arg); return *this; } \
      operator TYPE() const { return SWAPPER(this->endianSwappedVal); } \
      private: TYPE endianSwappedVal; }
#else
  // we are Big Endian
  #define BIG_ENDIAN_DECL(TYPE, SWAPPER) typedef TYPE TYPE##be
  #define LITTLE_ENDIAN_DECL(TYPE, SWAPPER) \
    class TYPE##le { public: \
      TYPE##le & operator = (TYPE arg) { this->endianSwappedVal = SWAPPER(arg); return *this; } \
      operator TYPE() const { return SWAPPER(this->endianSwappedVal); } \
      private: TYPE endianSwappedVal; }
#endif

//LITTLE_ENDIAN_DECL(uint64, OSSwapConstInt64);			// uint64le
LITTLE_ENDIAN_DECL(uint32, OSSwapConstInt32);			// uint32le
LITTLE_ENDIAN_DECL(int32, OSSwapConstInt32);			// int32le
LITTLE_ENDIAN_DECL(uint16, OSSwapConstInt16);			// uint16le
LITTLE_ENDIAN_DECL(int16, OSSwapConstInt16);			// int16le
//BIG_ENDIAN_DECL(uint64, OSSwapConstInt64);			// uint64be
BIG_ENDIAN_DECL(uint32, OSSwapConstInt32);			// uint32be
BIG_ENDIAN_DECL(int32, OSSwapConstInt32);			// int32be
BIG_ENDIAN_DECL(uint16, OSSwapConstInt16);			// uint16be
BIG_ENDIAN_DECL(int16, OSSwapConstInt16);			// int16be

/* Partition Map Entry */
struct MacPart {
  char                 pmSig[2];               /* unique value for map entry blk */
  int16be              pmSigPad;               /* currently unused */
  int32be              pmMapBlkCnt;            /* # of blks in partition map */
  int32be              pmPyPartStart;          /* physical start blk of partition */
  int32be              pmPartBlkCnt;           /* # of blks in this partition */
  char                 pmPartName[32];         /* ASCII partition name */
  char                 pmParType[32];          /* ASCII partition type */
  int32be              pmLgDataStart;          /* log. # of partition's 1st data blk */
  int32be              pmDataCnt;              /* # of blks in partition's data area */
  int32be              pmPartStatus;           /* bit field for partition status */
  int32be              pmLgBootStart;          /* log. blk of partition's boot code */
  int32be              pmBootSize;             /* number of bytes in boot code */
  int32be              pmBootAddr;             /* memory load address of boot code */
  int32be              pmBootAddr2;            /* currently unused */
  int32be              pmBootEntry;            /* entry point of boot code */
  int32be              pmBootEntry2;           /* currently unused */
  int32be              pmBootCksum;            /* checksum of boot code */
  uint8                pmProcessor[16];        /* ASCII for the processor type */
  uint8                pmPad[376];
};

static void hfsplus_newfs (uint8 part, uint32 offset);

static uint8 *gBlkBuf = 0;


extern "C" void check_mac_partitions (uint8 *blk0)
/*
 * used references:
 *	http://developer.apple.com/technotes/tn/tn1189.html#SecretsOfThePartitionMap
 */
{
	int blkNo = 1; // first part map entry block number
	int partBlkCount = 1; // number of part map blocks - we will update it below once we know the proper value
	int err;
	
	if (sizeof (MacPart) != 512) {
		mlc_printf ("!Internal err: macpart size: %d\n", sizeof (MacPart));
		mlc_show_critical_error();
		return;
	}
	
    if (!gBlkBuf) gBlkBuf = (uint8*) mlc_malloc (512);
    
	int partBlkSizMul = blk0[2] / 2;	// = size of partition blocks times 512
	
	while (blkNo <= partBlkCount) {
		MacPart *pm = (MacPart*) gBlkBuf;
		
		// read next block
		err = ata_readblock (gBlkBuf, blkNo * partBlkSizMul);
		if (err) {
			mlc_printf ("!Read error blk %d: %d\n", blkNo * partBlkSizMul, err);
			mlc_show_critical_error();
			break; // read error -> leave the loop
		}
		
		// see if it's a partition entry
		if (pm->pmSig[0] != 'P' || pm->pmSig[1] != 'M') break;	// end of partition table -> leave the loop
		
		// update the number of part map blocks
		partBlkCount = pm->pmMapBlkCnt;
		long partBlk = pm->pmPyPartStart * partBlkSizMul;
		
		#if DEBUG
			mlc_printf ("part name: %s, type: %s\n", pm->pmPartName, pm->pmParType);
		#endif
		
		// check the partition type so we can detect the firmware and HFS+ partitions
		if (0 == mlc_strncmp (pm->pmParType, "Apple_MDFW", sizeof (pm->pmParType))) {
			// a firmware partition
			#if DEBUG
				mlc_printf ("found firmware partition\n", pm->pmPartName, pm->pmParType);
			#endif
			fwfs_newfs (blkNo-2, partBlk);
		} else if (0 == mlc_strncmp (pm->pmParType, "Apple_HFS", sizeof (pm->pmParType))) {
			// a HFS(+) partition
			#if DEBUG
				mlc_printf ("found HFS partition\n", pm->pmPartName, pm->pmParType);
			#endif
			hfsplus_newfs (blkNo-2, partBlk);
		} else {
			// something else - let's ignore it for now
		}
		
		blkNo++; // read next partition map entry
	}

	#if DEBUG
		mlc_printf ("End of partition map\n");
	#endif
}

//----------------------------------------------------------------------------------
//  HFS+ Code
//----------------------------------------------------------------------------------

#include "hfsplusstructs.h"


// what follows is code for comparing unicode strings with the rules Apple has defined
// for use with names in the catalog (i.e. order of characters, upper/lower case conversion)

#include "unicodecmp.h"

static int compareUnicode (const hfsunistr s1, const hfsunistr s2)
{
	return FastUnicodeCompare (&s1.unicode[0], s1.length, &s2.unicode[0], s2.length);
}


#define MAX_HANDLES 10

#define ExtentCnt 8	// do not change this - it is fixed by HFS+
typedef ext_long ext_set[ExtentCnt];


static int fileHasOverflownExtents (forkdata* theFork, int showError = 0, const char* name = 0)
{
	// check whether the 8 extents in the dir entry cover the entire file
	// (if not, we have a problem because this code does not use the
	// extents overflow file yet)
	uint32 clusterCnt = 0;
	for (int i = 0; i < ExtentCnt; ++i) {
		clusterCnt += theFork->extents[i].blockCount;
	}
	if (clusterCnt != theFork->totalBlocks) {
		// we have a problem
		if (showError) {
			mlc_printf ("!Error: too many extents in: %s\n", name ? name : "?");
		}
		return 1;
	}
	// all is fine
	return 0;
}


typedef struct {
	ext_set fileExtents;
	int32  length;
	uint32 position;
	char   opened;
} hfsplus_file;

typedef struct {
	// constant values from the MDB:
	uint32		catNodeSize;
	ext_set		catExtents;
	uint32		partBlkStart;
	uint32		partClusterSize;
	uint32		blksInACluster;
	uint32		catRootNodeID;

	// dynamic values for the file management:
	hfsplus_file *filehandles[MAX_HANDLES];
	uint32 numHandles;
} hfsplus_t;


static void* nodeBuf = 0;
static uint32 nodeBufSize = 0;
static uint32 nodeBufID = (uint32)-1;
static char nodeBufInUse = 0;
static ext_set* gCurrExtents = 0;
static hfsplus_t* gCurrVolume = 0;

static uint32 nodeToBlockNo (uint32 id)
{
	uint32 nodeCluster = id * gCurrVolume->catNodeSize;
	// first, find the extent containing the given node number
	for (int i = 0; i < ExtentCnt; ++i) {
		if ((id* gCurrVolume->catNodeSize) < ((*gCurrExtents)[i].blockCount * gCurrVolume->partClusterSize)) {
			// found the extent
			uint32 blockNo = ((*gCurrExtents)[i].startBlock * gCurrVolume->partClusterSize + id * gCurrVolume->catNodeSize) / 512 + gCurrVolume->partBlkStart;
			return blockNo;
		}
		nodeCluster -= ((*gCurrExtents)[i].blockCount * gCurrVolume->partClusterSize);
	}
	mlc_printf ("!Error: extents overflow\n");
	mlc_show_critical_error();
	return 0;
}

static hfs_node* getNode (uint32 id)
// id is the catalog file's cluster number of the node
{
	if (nodeBufInUse) {
		mlc_printf ("!Internal err: getNode - node in use\n");
		mlc_show_critical_error();
		return 0;
	}
	if (nodeBufSize < gCurrVolume->catNodeSize) {
		// we need a larger node buffer
		nodeBufSize = gCurrVolume->catNodeSize;
		nodeBuf = mlc_malloc (nodeBufSize);
		nodeBufID = (uint32)-1;
	}
	if (!nodeBuf) {
		mlc_printf ("!Internal err: getNode - out of mem\n");
		mlc_show_critical_error();
		return 0;
	}
	nodeBufInUse = 1;

	if (nodeBufID == id) {
		// we have this blk still in the buffer, no need to read it again
	} else {
		uint32 blkNo = nodeToBlockNo (id);
		ata_readblocks (nodeBuf, blkNo, gCurrVolume->catNodeSize / 512);
		nodeBufID = id;
	}
	return (hfs_node*) nodeBuf;
}

static void releaseNode (hfs_node* node)
{
	nodeBufInUse = 0;
}


typedef void* recptr;

static uint16 hfsRecofs (hfs_node *node, short i)
{
	return ((uint16be*)node)[(gCurrVolume->catNodeSize/2-1) - i];
}

static int16be* getRecord(hfs_node *node, short i) 
{ 
	return (int16be*)(((char*)node) + hfsRecofs(node, i));
}

static int compareKey (const recptr key1, const recptr key2)
{
	int result = ((cat_key*)key1)->parentID - ((cat_key*)key2)->parentID;
	if (result == 0) {
		result = compareUnicode (((cat_key*)key1)->nodeName, ((cat_key*)key2)->nodeName);
	}
	return result;
}

static uint16 keyLen (const recptr key)
{
	return 2 + *(uint16be*)key;
}

static recptr skipKey (const recptr key)
{
	return (recptr)((char*)key + keyLen(key));
}

static recptr searchLeafNode(hfs_node *node, const recptr key)
{
	short n = node->numRecords;
	for (short i = 0; i < n; i++) {
		recptr rec = getRecord (node, i);
		int result = compareKey (key, rec);
		if (result == 0) {
			return skipKey(rec);
		}
		if (result < 0) {
			break;
		}
	}
	return NULL;
}

static int32 searchIndexNode(hfs_node *node, const recptr key)
{
	int32 nextNode = 0;
	for (short i = 0; i < node->numRecords; i++) {
		recptr rec = getRecord (node, i);
		int32 nodeID = (int32) *((int32be *) skipKey(rec));
		int result = compareKey (key, rec);
		if (result < 0) {
			if (nextNode == 0) nextNode = nodeID;
			break;
		}
		nextNode = nodeID;
	}
	return nextNode;
}

static recptr searchNode(uint32 nodeID, const recptr key)
{
	hfs_node *node = getNode (nodeID);
	recptr	result = NULL;
	if (nodeID) {
		if (node->type == kIndexNode) {
			nodeID = searchIndexNode (node, key);
			releaseNode (node); node = (hfs_node*)NULL; // this makes sure we do not keep more than one node open at all times
			result = searchNode(nodeID, key);
		} else {
			result = searchLeafNode (node, key);
		}
	}
	if (node) releaseNode (node);	// attn: we release it here, yet we will still access the buffer!
	return result;
}

static recptr findkey (const recptr key)
{
	return searchNode (gCurrVolume->catRootNodeID, key);
}

static void hfsglobals_enter (hfsplus_t* fsData, ext_set* extents)
{
	if (gCurrExtents) {
		mlc_printf ("!Internal err: gCurrExtents in use\n");
		mlc_show_critical_error();
	}
	gCurrExtents = extents;
	gCurrVolume = fsData;
}

static void hfsglobals_leave ()
{
	gCurrExtents = 0;
	gCurrVolume = 0;
}

static cat_data_rec* findCatalogData (hfsplus_t* fsData, long parID, const hfsunistr *name)
{
	hfsglobals_enter (fsData, &fsData->catExtents);
	cat_key	key;
	key.parentID = parID;
	key.nodeName = *name;	// for some reason, using mlc_memcpy here instead leads to a crash
	key.keyLength = 4 + 2 + 2 * key.nodeName.length;
	cat_data_rec *rec = (cat_data_rec*) findkey ((uint8*)&key);
	hfsglobals_leave ();
	return rec;
}

static void getExtent (hfsplus_t* fsData, ext_set &extents, uint32 position, uint32 *blockOut, uint32 *ofsInBlkOut, uint32 *remBytesInExtOut)
{
	uint32 clusterNo = position / fsData->partClusterSize;
	position -= clusterNo * fsData->partClusterSize;
	int i;
	uint32 clustersInExt;
	for (i = 0; i < ExtentCnt; ++i) {
		clustersInExt = extents[i].blockCount;
		if (clusterNo < clustersInExt) break;	// found the extent
		clusterNo -= clustersInExt;
	}
	*remBytesInExtOut = (clustersInExt - clusterNo) * fsData->partClusterSize - position;
	// now we have the cluster's start, but we want to get to the block's (512 byte size) start
	uint32 remBlks = position / 512;
	position -= remBlks * 512;
	*blockOut = (extents[i].startBlock + clusterNo) * fsData->blksInACluster + fsData->partBlkStart + remBlks;
	*ofsInBlkOut = position;
}


// -----------------------------
//         vfs handlers
// -----------------------------

static hfsplus_file *hfsplus_findfile (hfsplus_t *fsdata, char *fname)
{
	cat_data_rec* cdat = 0;
	long parID = 2; // root dir
	char *origName = fname;
	char name[256];

	if (fname && *fname == '/') ++fname;
	
	while (fname && *fname) {
		
		if (cdat) {
			if (cdat->d.recordType != kHFSPlusFolderRecord) {
				// last segment was not a folder
				mlc_printf ("!Oops: not a folder: %s\n", name);
				mlc_show_critical_error ();
				return 0;
			}
			parID = cdat->d.folderID;
		}
		
		// extract the next path segment
		char *nextPath;
		int len;
		nextPath = mlc_strchr (fname,'/');
		if (nextPath) {
			len = nextPath - fname;
			nextPath++;
		} else {
			len = mlc_strlen (fname);
		}
		mlc_memcpy (name, fname, len);
		name[len] = 0;
		
		// locate the dir entry
		hfsunistr uname;
		uname = name;
		cdat = findCatalogData (fsdata, parID, &uname);
		if (!cdat) {
			// not found
			return 0;
		}
		
		fname = nextPath;
	}

	if (cdat->f.recordType != kHFSPlusFileRecord) {
		// found, but it's not a file
		mlc_printf ("!Oops: not a file: %s\n", origName);
		mlc_show_critical_error ();
		return 0;
	}

	if (fileHasOverflownExtents (&cdat->f.dataFork, 1, origName)) {
		mlc_show_critical_error();
		return 0;
	}

	hfsplus_file *fileptr = 0;
	fileptr = (hfsplus_file*)mlc_malloc (sizeof(hfsplus_file));

	fileptr->length = cdat->f.dataFork.logicalSizeLo;
	fileptr->position = 0;
	
	// we need to copy the extents, but this code leads to a crash:
	//	mlc_memcpy (&fileptr->fileExtents, cdat->f.dataFork.extents, sizeof (fileptr->fileExtents));
	// so we copy it by hand instead:
	for (int i = 0; i < ExtentCnt; ++i) { fileptr->fileExtents[i] = cdat->f.dataFork.extents[i]; }

	return fileptr;
}

static int hfsplus_open (void *fsdata, char *fname)
{
	hfsplus_file *file = 0;
	hfsplus_t *fs;

	fs = (hfsplus_t*)fsdata;

	#if DEBUG
		mlc_printf ("### hfs+: looking for %s ###\n", fname);
	#endif

	file = hfsplus_findfile (fs, fname);

	if (!file) {
		#if DEBUG
			mlc_printf ("  NOT found\n");
		#endif
		return -1;
	}

	#if DEBUG
		mlc_printf ("  found OK\n");
	#endif

	if (file != NULL) {
		if (fs->numHandles < MAX_HANDLES) {
			fs->filehandles[fs->numHandles] = file;
			return fs->numHandles++;
		} else {
			mlc_printf ("!Internal err: out of file hdls\n");
			mlc_show_critical_error();
		}
	}
	
	return -1;
}

static void hfsplus_close (void *fsdata, int fd)
{
	hfsplus_t *fs = (hfsplus_t*)fsdata;
	if (fd == (int)fs->numHandles-1) {
		--fs->numHandles;
	}
}

static void copyBytesFromTo (const char* from, char* to, long n)
{
	while (n-- > 0) { *to++ = *from++; }
}

static size_t hfsplus_read (void *fsdata, void *ptr, size_t size, size_t nmemb, int fd)
{
	hfsplus_t *fs = (hfsplus_t*)fsdata;
	hfsplus_file *fh = fs->filehandles[fd];

	uint32 totalRead, toRead;
	totalRead = 0;
	toRead = size*nmemb;
	uint32 filePos = fh->position;
	if (toRead > (fh->length + filePos)) {
		toRead = fh->length + filePos;
	}
	
	while (toRead > 0) {
		uint32 blockNum, ofsInBlk, remBytesInExtent;
		getExtent (fs, fh->fileExtents, filePos, &blockNum, &ofsInBlk, &remBytesInExtent);
		while (toRead > 0 && remBytesInExtent > 0) {
			uint32 bytesInBlk = 512 - ofsInBlk;
			if (bytesInBlk > toRead) bytesInBlk = toRead;
			if (bytesInBlk != 512 || ((uint32)ptr & 3) != 0) {
				// copy using an interim buffer
				if (bytesInBlk == 512) {
					#if DEBUG
						mlc_printf ("## hfs warning: slow read\n");
					#endif
					ata_readblocks_uncached (gBlkBuf, blockNum, 1);	// uncached read for whole blocks
				} else {
					ata_readblocks (gBlkBuf, blockNum, 1);	// cached read for partial blocks
				}
				copyBytesFromTo ((char*)gBlkBuf + ofsInBlk, (char*)ptr, bytesInBlk);
			} else {
				// load the data directly to the destination
				ata_readblocks_uncached (ptr, blockNum, 1);
			}
			ofsInBlk = 0;
			ptr = (char*)ptr + bytesInBlk;
			remBytesInExtent -= bytesInBlk;
			toRead -= bytesInBlk;
			filePos += bytesInBlk;
			++blockNum;
			totalRead += bytesInBlk;
		}
	}
	
	fh->position += totalRead;
	return totalRead / size;
}

static long hfsplus_tell (void *fsdata,int fd)
{
	hfsplus_t *fs = (hfsplus_t*)fsdata;
	return fs->filehandles[fd]->position;
}

static int hfsplus_seek (void *fsdata,int fd,long offset,int whence)
{
	hfsplus_t *fs = (hfsplus_t*)fsdata;
	
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
		mlc_printf ("!Internal err: wrong seek whence: %d\n", whence);
		mlc_show_critical_error();
		return -2;
	}

	if( offset < 0 || offset > fs->filehandles[fd]->length ) {
		return -1;
	}

	fs->filehandles[fd]->position = offset;
	return 0;
}


// -----------------------------
//       vfs installation
// -----------------------------

static filesystem myfs;

#define assert_size(s,t) if (s != sizeof (t)) { mlc_printf ("!Internal err: wrong struct size\n"); mlc_show_critical_error(); }

static void hfsplus_newfs (uint8 part, uint32 offset) {
	hfsplus_mdb* mdb = (hfsplus_mdb*) gBlkBuf;

	assert_size (106, btree_hdr);

	/* Verify that this is a hfs+ partition */
	ata_readblock (gBlkBuf, offset+2);
	if ((gBlkBuf[0] != 'H') || (gBlkBuf[1] != '+')) {
		mlc_printf ("!Error: not a valid HFS+ partition\n");
		mlc_show_critical_error ();
		return;
	}

	if (fileHasOverflownExtents (&mdb->catalogFile, 1, "HFS Catalog File")) {
		mlc_show_critical_error();
		return;
	}
	
	/* allocate the storage for the fs data (so that we can handle more than one HFS partition) */
	hfsplus_t* fsData = (hfsplus_t*) mlc_malloc (sizeof (hfsplus_t));
	if (!fsData) {
		mlc_printf ("!Error: hfsplus_newfs - out of mem\n");
		mlc_show_critical_error();
		return;
	}
	myfs.open	= hfsplus_open;
	myfs.close	= hfsplus_close;
	myfs.tell	= hfsplus_tell;
	myfs.seek	= hfsplus_seek;
	myfs.read	= hfsplus_read;
	myfs.getinfo	= 0;
	myfs.fsdata	= (void*)fsData;
	myfs.partnum	= part;
	myfs.type	= HFSPLUS;

	/* set up the fs data for this partition */
	fsData->numHandles = 0;
	fsData->partBlkStart = offset;
	fsData->partClusterSize = mdb->blockSize;
	fsData->blksInACluster = fsData->partClusterSize / 512;
	for (int i = 0; i < ExtentCnt; ++i) { fsData->catExtents[i] = mdb->catalogFile.extents[i]; }
	fsData->catNodeSize = 8192;	// will be updated below

	// get the btree root node
	gCurrVolume = fsData;
	gCurrExtents = &fsData->catExtents;
	hfs_node *node = getNode (0);
	btree_hdr *hdr = (btree_hdr*) &node->data[0];
	{
		fsData->catNodeSize = hdr->nodeSize;
		fsData->catRootNodeID = hdr->rootNodeID;
	}
	releaseNode (node);	// attn: we release it here, yet we will still access the buffer!
	gCurrVolume = 0;
	gCurrExtents = 0;
	
	vfs_registerfs (&myfs);
	
/*
	{	// test:
		int fd = hfsplus_open (fsData, "macpartitions.cc");
		if (fd >= 0) {
			char buff[2024];
			long n;
			n = hfsplus_read (fsData, buff, 100, 1, fd);
			n = hfsplus_read (fsData, buff, 2000, 1, fd);
			n = hfsplus_read (fsData, buff, 100, 1, fd);
		}
	}
*/
}

// EOF
