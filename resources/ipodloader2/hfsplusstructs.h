
// these structures come from Darwin, hfs_format.h

#pragma pack (1)

enum {
	kHFSMaxVolumeNameChars		= 27,
	kHFSMaxFileNameChars		= 31,
	kHFSPlusMaxFileNameChars	= 255,

	kHFSPlusFolderRecord		= 1,
	kHFSPlusFileRecord			= 2,
	kHFSPlusFolderThreadRecord	= 3,
	kHFSPlusFileThreadRecord	= 4
};

class hfsunistr {
public:
	uint16be	length;
	uint16be	unicode[255];
	hfsunistr& operator = (const char* str) {
		int n = mlc_strlen (str);
		this->length = n;
		for (int i = 0; i < n; ++i) {
			this->unicode[i] = (unsigned char)str[i];
		}
		return *this; 
	}
};

class ext_short {
public:
	uint16be		startBlock;
	uint16be		numBlocks;
};

typedef ext_short	extrec_short[3];

class ext_long {
public:
	uint32be	startBlock;
	uint32be	blockCount;
};

class mdb_desc {
public:
	uint16be 		drSigWord;
	uint32be 		drCrDate;
	uint32be 		drLsMod;
	uint16be 		drAtrb;
	uint16be 		drNmFls;
	uint16be 		drVBMSt;
	uint16be 		drAllocPtr;
	uint16be 		drNmAlBlks;
	uint32be 		drAlBlkSiz;
	uint32be 		drClpSiz;
	uint16be 		drAlBlSt;
	uint32be 		drNxtCNID;
	uint16be 		drFreeBks;
	uint8			drVN[kHFSMaxVolumeNameChars + 1];
	uint32be 		drVolBkUp;
	uint16be 		drVSeqNum;
	uint32be 		drWrCnt;
	uint32be 		drXTClpSiz;
	uint32be 		drCTClpSiz;
	uint16be 		drNmRtDirs;
	uint32be 		drFilCnt;
	uint32be 		drDirCnt;
	uint32be 		drFndrInfo[8];
	uint16be 		drEmbedSigWord;
	ext_short		drEmbedExtent;
	uint32be		drXTFlSize;
	extrec_short	drXTExtRec;
	uint32be 		drCTFlSize;
	extrec_short	drCTExtRec;
};

class forkdata {
public:
	uint32be 	logicalSizeHi;
	uint32be 	logicalSizeLo;
	uint32be 	clumpSize;
	uint32be 	totalBlocks;
	ext_long	extents[8];
};

class hfsplus_mdb {
public:
	uint16be 	signature;
	uint16be 	version;
	uint32be 	attributes;
	uint32be 	lastMountedVersion;
	uint32be 	journalInfoBlock;
	uint32be 	createDate;
	uint32be 	modifyDate;
	uint32be 	backupDate;
	uint32be 	checkedDate;
	uint32be 	fileCount;
	uint32be 	folderCount;
	uint32be 	blockSize;
	uint32be 	totalBlocks;
	uint32be 	freeBlocks;
	uint32be 	nextAllocation;
	uint32be 	rsrcClumpSize;
	uint32be 	dataClumpSize;
	uint32be 	nextCatalogID;
	uint32be 	writeCount;
	uint32be 	encodingsBitmapHi;
	uint32be 	encodingsBitmapLo;
	uint8		finderInfoBE[32];
	forkdata	allocationFile;
	forkdata	extentsFile;
	forkdata	catalogFile;
	forkdata	attributesFile;
	forkdata	startupFile;
};

typedef struct bsd_info {
	uint32be 	ownerID;
	uint32be 	groupID;
	uint8		adminFlags;
	uint8		ownerFlags;
	uint16be 	fileMode;
	union {
	    uint32be	iNodeNum;
	    uint32be	linkCount;
	    uint32be	rawDevice;
	} special;
} bsd_info;

typedef struct cat_folder {
	int16be 	recordType;		// == kHFSPlusFolderRecord
	uint16be	flags;
	uint32be	valence;
	uint32be	folderID;
	uint32be	createDate;
	uint32be	contentModDate;
	uint32be	attributeModDate;
	uint32be	accessDate;
	uint32be	backupDate;
	bsd_info	bsdInfo;
	char 		userInfo[16];
	char	 	finderInfo[16];
	uint32be	textEncoding;
	uint32be	attrBlocks;
} cat_folder;

typedef struct cat_file {
	int16be 	recordType;		/* == kHFSPlusFileRecord */
	uint16be	flags;
	uint32be	reserved1;
	uint32be	fileID;
	uint32be	createDate;
	uint32be	contentModDate;
	uint32be	attributeModDate;
	uint32be	accessDate;
	uint32be	backupDate;
	bsd_info	bsdInfo;
	char 		userInfo[16];
	char		finderInfo[16];
	uint32be	textEncoding;
	uint32be	attrBlocks;

	forkdata 	dataFork;		/* size and block data for data fork */
	forkdata 	resourceFork;		/* size and block data for resource fork */
} cat_file;

typedef struct cat_thread {
	int16be 	recordType;		/* == kHFSPlusFolderThreadRecord or kHFSPlusFileThreadRecord */
	int16be 	reserved;		/* reserved - initialized as zero */
	uint32be 	parentID;		/* parent ID for this catalog node */
	hfsunistr 	nodeName;		/* name of this catalog node (variable length) */
} cat_thread;

union cat_data_rec {
	cat_folder	d;
	cat_file	f;
	cat_thread	t;
};

typedef struct cat_key {
	uint16be 	keyLength;
	uint32be 	parentID;
	hfsunistr 	nodeName;
} cat_key;

enum {
	// BTree node types
	kIndexNode = 0x00,
	kHeaderNode = 0x01,
	kMapNode = 0x02,
	kLeafNode = 0xFF
};

struct hfs_node {			// B*-Tree node
	uint32be	next;
	uint32be	prev;
	uint8		type;
	uint8		level;
	uint16be	numRecords;
	int16		reserved1;
	int16be		data[];
};

struct btree_hdr {	// B*-Tree header record
	int16be		depth;
	int32be		rootNodeID;
	int32be		numLeafRecords;
	int32be		firstLeafNodeID;
	int32be		lastLeafNodeID;
	int16be		nodeSize;
	int16be		keyLen;
	uint32be	numNodes;
	uint32be	numFreeNodes;
	uint16be	reserved1;
	uint32be	clumpSize;
	uint8		btreeType;
	uint8		reserved2;
	uint32be	attributes;
	uint32		reserved3[16];
};

// EOF
