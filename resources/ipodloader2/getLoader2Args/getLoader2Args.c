/*
Author: Thomas Tempelmann (http://ipodlinux.org/User:Tempel)
Last change: 30Mar06

getLoader2Args is a tool that runs on the iPod under iPodLinux.
It is used in conjunction with iPodLoader2 (http://ipodlinux.org/Loader_2)

See the "readme.txt" for more info
*/

#include <stdio.h>
#include <unistd.h>
#include <string.h>

static short calc_checksum2 (char* dest, int size) {
  short csum = 0;
  while (size-- > 0) {
    char b = *dest++;
    csum = ((csum << 1) & 0xffff) + ((csum<0)? 1 : 0) + b; // csum-rotation plus b
  }
  return csum;
}

static char* getArgs (char* baseAddr) {
  // fetch the args
  if (strncmp (baseAddr, "Args", 4) == 0) {
    int strlen = *(short*)(baseAddr+6);
    if (*(short*)(baseAddr+4) == calc_checksum2 (baseAddr+6, strlen+2)) {
      return baseAddr + 8;
    }
  }
  return 0;
}

/*
static void memdump (long addr, int len) {
  int i;
  while (len > 0) {
    for (i = 0; i < 4; ++i) {
      printf (" %08x", *(long*)(addr));
      addr += 4;
      len -= 4;
    }
    printf ("\n");
  }
}
*/

int main (int argc, char **argv) 
{
/* look for non-empty spaces:
  long *p = 0x24;
  while (p < (long*)32768) {
    if (*p++) {
      printf (" %08lx", (long)p-4);
      while (*p++) { }
      printf ("-%08lx", (long)p);
    }
  }
  printf ("\n");
  printf ("end: %08lx", (long)p);
  printf ("\n");
  
  memdump (0x80, 0x20);
*/
  char *args;

  args = getArgs ((char*)0x80);
  
  if (args) {
    puts (args);
  }
  
  return 0;
}
