/*
 * macpartitions.h
 *
 * part of "ipodloader2" of the iPodLinux project
 *
 * purpose: read a MacPod's partition table and create file system handlers
 *          invoked from vfs.c
 *
 * written by Thomas Tempelmann (http://ipodlinux.org/User:Tempel)
 */

void check_mac_partitions (uint8 *blk0);
