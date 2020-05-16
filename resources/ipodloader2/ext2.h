#ifndef _EXT2_H_
#define _EXT2_H_

#include "bootloader.h"

typedef struct {
  uint32 s_inodes_count;
  uint32 s_blocks_count;
  uint32 s_r_blocks_count;
  uint32 s_free_blocks_count;
  uint32 s_free_inodes_count;
  uint32 s_first_data_block;
  uint32 s_log_block_size;
  uint32 s_log_frag_size;
  uint32 s_blocks_per_group;
  uint32 s_frags_per_group;
  uint32 s_inodes_per_group;
  uint32 s_mtime;
  uint32 s_wtime;
  uint16 s_mnt_count;
  uint16 s_max_mnt_count;
  uint16 s_magic;
  uint16 s_state;
  uint16 s_errors;
  uint16 s_minor_rev_level;
  uint32 s_lastcheck;
  uint32 s_checkinterval;
  uint32 s_creator_os;
  uint32 s_rev_level;
  uint16 s_def_resuid;
  uint16 s_def_resqid;
  uint32 s_first_ino;
  uint16 s_inode_size;
  uint16 s_block_group_nr;
  uint32 s_feature_compat;
  uint32 s_feature_incompat;
  uint32 s_feature_ro_compat;
  uint8  s_uuid[16];
  uint8  s_volume_name[16];
  uint8  s_last_mounted[64];
  uint32 s_algo_bitmap;
  uint8  s_prealloc_blocks;
  uint8  s_prealloc_dir_blocks;
  uint16 align;
  uint8  s_journal_uuid[16];
  uint32 s_journal_inum;
  uint32 s_journal_dev;
  uint32 s_last_orphan;
  uint8  padding[788];
} superblock_t;

typedef struct {
  uint16 i_mode;
  uint16 i_uid;
  uint32 i_size;
  uint32 i_atime;
  uint32 i_ctime;
  uint32 i_mtime;
  uint32 i_dtime;
  uint16 i_gid;
  uint16 i_links_count;
  uint32 i_blocks;
  uint32 i_flags;
  uint32 i_osdl;
  uint32 i_block[15];
  uint32 i_generation;
  uint32 i_file_acl;
  uint32 i_dir_acl;
  uint32 i_faddr;
  uint8  i_osd2[12];
} inode_t;

typedef struct {
  uint32 inode;
  uint16 rec_len;
  uint8  name_len;
  uint8  file_type;
  uint8  name[255];
} dir_t;

typedef struct _group_desc {
  uint32 bg_block_bitmap;
  uint32 bg_inode_bitmap;
  uint32 bg_inode_table;
  uint16 bg_free_blocks_count;
  uint16 bg_free_inodes_count;
  uint16 bg_used_dirs_count;
  uint16 bg_pad;
  uint8  bg_reserved[12];
} group_t;

typedef struct {
  inode_t inode;
  uint32  inodeNum;
  uint32  length;
  uint32  opened;
  uint32  position;
} ext2_file;

void ext2_newfs(uint8 part,uint32 offset);

#endif
