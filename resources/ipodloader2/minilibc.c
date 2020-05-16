/*****************************************************************************
Stripped-down printf()
Chris Giese <geezer@execpc.com>	http://www.execpc.com/~geezer
Release date: Dec 12, 2003
This code is public domain (no copyright).
You can do whatever you want with it.

Revised Dec 12, 2003
- fixed vsprintf() and sprintf() in test code

Revised Jan 28, 2002
- changes to make characters 0x80-0xFF display properly

Revised June 10, 2001
- changes to make vsprintf() terminate string with '\0'

Revised May 12, 2000
- math in DO_NUM is now unsigned, as it should be
- %0 flag (pad left with zeroes) now works
- actually did some TESTING, maybe fixed some other bugs

%[flag][width][.prec][mod][conv]
flag:	-	left justify, pad right w/ blanks	DONE
	0	pad left w/ 0 for numerics		DONE
	+	always print sign, + or -		no
	' '	(blank)					no
	#	(???)					no

width:		(field width)				DONE

prec:		(precision)				no

conv:	d,i	decimal int				DONE
	u	decimal unsigned			DONE
	o	octal					DONE
	x,X	hex					DONE
	f,e,g,E,G float					no
	c	char					DONE
	s	string					DONE
	p	ptr					DONE

mod:	N	near ptr				DONE
	F	far ptr					no
	h	short (16-bit) int			DONE
	l	long (32-bit) int			DONE
	L	long long (64-bit) int			no
*****************************************************************************/

#if ONPC
	// for debugging on Mac OS or Windows or Linux or whatever
	#include <stdlib.h>
	#include <stdio.h>
#else
	#include "console.h"
	#include "ipodhw.h"
#endif

#include "fb.h"
#include "ipodhw.h"
#include "minilibc.h"
#include "interrupts.h"
#include "keypad.h"

/* flags used in processing format string */
#define	PR_LJ	0x01	/* left justify */
#define	PR_CA	0x02	/* use A-F instead of a-f for hex */
#define	PR_SG	0x04	/* signed numeric conversion (%d vs. %u) */
#define	PR_32	0x08	/* long (32-bit) numeric conversion */
#define	PR_16	0x10	/* short (16-bit) numeric conversion */
#define	PR_WS	0x20	/* PR_SG set and num was < 0 */
#define	PR_LZ	0x40	/* pad left with '0' instead of ' ' */
#define	PR_FP	0x80	/* pointers are far */
/* largest number handled is 2^32-1, lowest radix handled is 8.
2^32-1 in base 8 has 11 digits (add 5 for trailing NUL and for slop) */
#define	PR_BUFLEN	16

typedef int (*fnptr_t)(unsigned c, void **helper);
/*****************************************************************************
name:	do_printf
action:	minimal subfunction for ?printf, calls function
	'fn' with arg 'ptr' for each character to be output
returns:total number of characters output
*****************************************************************************/
static int mlc_do_printf(const char *fmt, mlc_va_list args, fnptr_t fn, void *ptr)
{
	unsigned flags, actual_wd, count, given_wd;
	unsigned char *where, buf[PR_BUFLEN];
	unsigned char state, radix;
	long num;

	state = flags = count = given_wd = 0;
/* begin scanning format specifier list */
	for(; *fmt; fmt++)
	{
		switch(state)
		{
/* STATE 0: AWAITING % */
		case 0:
			if(*fmt != '%')	/* not %... */
			{
				fn(*fmt, &ptr);	/* ...just echo it */
				count++;
				break;
			}
/* found %, get next char and advance state to check if next char is a flag */
			state++;
			fmt++;
			/* FALL THROUGH */
/* STATE 1: AWAITING FLAGS (%-0) */
		case 1:
			if(*fmt == '%')	/* %% */
			{
				fn(*fmt, &ptr);
				count++;
				state = flags = given_wd = 0;
				break;
			}
			if(*fmt == '-')
			{
				if(flags & PR_LJ)/* %-- is illegal */
					state = flags = given_wd = 0;
				else
					flags |= PR_LJ;
				break;
			}
/* not a flag char: advance state to check if it's field width */
			state++;
/* check now for '%0...' */
			if(*fmt == '0')
			{
				flags |= PR_LZ;
				fmt++;
			}
			/* FALL THROUGH */
/* STATE 2: AWAITING (NUMERIC) FIELD WIDTH */
		case 2:
			if(*fmt >= '0' && *fmt <= '9')
			{
				given_wd = 10 * given_wd +
					(*fmt - '0');
				break;
			}
/* not field width: advance state to check if it's a modifier */
			state++;
			/* FALL THROUGH */
/* STATE 3: AWAITING MODIFIER CHARS (FNlh) */
		case 3:
			if(*fmt == 'F')
			{
				flags |= PR_FP;
				break;
			}
			if(*fmt == 'N')
				break;
			if(*fmt == 'l')
			{
				flags |= PR_32;
				break;
			}
			if(*fmt == 'h')
			{
				flags |= PR_16;
				break;
			}
/* not modifier: advance state to check if it's a conversion char */
			state++;
			/* FALL THROUGH */
/* STATE 4: AWAITING CONVERSION CHARS (Xxpndiuocs) */
		case 4:
			where = buf + PR_BUFLEN - 1;
			*where = '\0';
			switch(*fmt)
			{
			case 'X':
				flags |= PR_CA;
				/* FALL THROUGH */
/* xxx - far pointers (%Fp, %Fn) not yet supported */
			case 'x':
			case 'p':
			case 'n':
				radix = 16;
				goto DO_NUM;
			case 'd':
			case 'i':
				flags |= PR_SG;
				/* FALL THROUGH */
			case 'u':
				radix = 10;
				goto DO_NUM;
			case 'o':
				radix = 8;
/* load the value to be printed. l=long=32 bits: */
DO_NUM:				if(flags & PR_32)
                                  num = mlc_va_arg(args, unsigned long);
/* h=short=16 bits (signed or unsigned) */
				else if(flags & PR_16)
				{
					if(flags & PR_SG)
						num = mlc_va_arg(args, short);
					else
						num = mlc_va_arg(args, unsigned short);
				}
/* no h nor l: sizeof(int) bits (signed or unsigned) */
				else
				{
					if(flags & PR_SG)
						num = mlc_va_arg(args, int);
					else
						num = mlc_va_arg(args, unsigned int);
				}
/* take care of sign */
				if(flags & PR_SG)
				{
					if(num < 0)
					{
						flags |= PR_WS;
						num = -num;
					}
				}
/* convert binary to octal/decimal/hex ASCII
OK, I found my mistake. The math here is _always_ unsigned */
				do
				{
					unsigned long temp;

					temp = (unsigned long)num % radix;
					where--;
					if(temp < 10)
						*where = (unsigned char)(temp + '0');
					else if(flags & PR_CA)
						*where = (unsigned char)(temp - 10 + 'A');
					else
						*where = (unsigned char)(temp - 10 + 'a');
					num = (unsigned long)num / radix;
				}
				while(num != 0);
				goto EMIT;
			case 'c':
/* disallow pad-left-with-zeroes for %c */
				flags &= ~PR_LZ;
				where--;
				*where = (unsigned char)mlc_va_arg(args,
					unsigned char);
				actual_wd = 1;
				goto EMIT2;
			case 's':
/* disallow pad-left-with-zeroes for %s */
				flags &= ~PR_LZ;
				where = mlc_va_arg(args, unsigned char *);
EMIT:
				actual_wd = (unsigned int)mlc_strlen((const char *)where);
				if(flags & PR_WS)
					actual_wd++;
/* if we pad left with ZEROES, do the sign now */
				if((flags & (PR_WS | PR_LZ)) ==
					(PR_WS | PR_LZ))
				{
					fn('-', &ptr);
					count++;
				}
/* pad on left with spaces or zeroes (for right justify) */
EMIT2:				if((flags & PR_LJ) == 0)
				{
					while(given_wd > actual_wd)
					{
						fn(flags & PR_LZ ?
							'0' : ' ', &ptr);
						count++;
						given_wd--;
					}
				}
/* if we pad left with SPACES, do the sign now */
				if((flags & (PR_WS | PR_LZ)) == PR_WS)
				{
					fn('-', &ptr);
					count++;
				}
/* emit string/char/converted number */
				while(*where != '\0')
				{
					fn(*where++, &ptr);
					count++;
				}
/* pad on right with spaces (for left justify) */
				if(given_wd < actual_wd)
					given_wd = 0;
				else given_wd -= actual_wd;
				for(; given_wd; given_wd--)
				{
					fn(' ', &ptr);
					count++;
				}
				break;
			default:
				break;
			}
		default:
			state = flags = given_wd = 0;
			break;
		}
	}
	return count;
}

#if 1 /* testing */
/*****************************************************************************
SPRINTF
*****************************************************************************/
static int mlc_vsprintf_help(unsigned c, void **ptr) {
	char *dst;

	dst = *ptr;
	*dst++ = (char)c;
	*ptr = dst;
	return 0 ;
}
/*****************************************************************************
*****************************************************************************/
static int mlc_vsprintf(char *buf, const char *fmt, mlc_va_list args)
{
	int rv;

	rv = mlc_do_printf(fmt, args, mlc_vsprintf_help, (void *)buf);
	buf[rv] = '\0';
	return rv;
}
/*****************************************************************************
*****************************************************************************/
int mlc_sprintf(char *buf, const char *fmt, ...) {
	mlc_va_list args;
	int rv;

	mlc_va_start(args, fmt);
	rv = mlc_vsprintf(buf, fmt, args);
	mlc_va_end(args);
	return rv;
}

/*****************************************************************************
PRINTF
You must write your own putchar()
*****************************************************************************/
static int print_to_console(unsigned c, void **ptr) {
  #if ONPC
    putchar(c);
  #else
    console_putchar(c);
  #endif
  return 0;
}

static int do_buffered_printf = 0;  // boolean flag
static int do_slow_printf = 0;      // boolean flag
static const int printf_buffer_size = 512; // 512 should be enough for what we use it for
static uint8 *printf_buffer = 0;
static int printf_buflen = 0;       // this is the used amount in the buffer

// alternative, used with buffered output
static int print_to_buffer(unsigned c, void **ptr) {
  if (printf_buflen >= printf_buffer_size) {
    // make room (discard older output)
    printf_buflen -= 40;
    mlc_memcpy (printf_buffer, printf_buffer+40, printf_buflen);
  }
  printf_buffer[printf_buflen++] = c;
  return 0;
}
/*****************************************************************************
*****************************************************************************/
int mlc_vprintf(const char *fmt, mlc_va_list args) {
	return mlc_do_printf(fmt, args, print_to_console, NULL);
}
/*****************************************************************************
*****************************************************************************/
int mlc_printf(const char *fmt, ...) {
  mlc_va_list args;
  int rv;

  mlc_va_start(args, fmt);
  if (do_buffered_printf) {
    rv = mlc_do_printf(fmt, args, print_to_buffer, NULL);
  } else {
    rv = mlc_vprintf(fmt, args);
  }
  mlc_va_end(args);
  if (do_slow_printf) {
    mlc_delay_ms (1000); // pause 1s - for debugging
  }
  return rv;
}
/*****************************************************************************
*****************************************************************************/
#endif

static uint32 malloc_top;

void mlc_malloc_init(void) {
  #if !ONPC
    ipod_t *hw = ipod_get_hwinfo ();
    malloc_top = hw->mem_base + hw->mem_size;
  #endif
}

void *mlc_malloc(size_t size) {
  #if ONPC
    return malloc (size);
  #else
    size = (size + 15) & 0xfffffff0; // round size up to a 4-byte boundary
    malloc_top -= size;
    return (void*) malloc_top;
  #endif
}

size_t mlc_strlen(const char *str) {
  uint32 i = 0;
  while (*str++) i++;
  return i;
}

int mlc_memcmp(const void *sv1,const void *sv2,size_t length) {
  uint8 *s1,*s2;

  s1 = (uint8*)sv1;
  s2 = (uint8*)sv2;

  while(length) {
    if(*s1 != *s2)
      return( (*s2 - *s1) );

    length--;
    s1++;
    s2++;
  }

  if(!length)
    return(0);

  return(-1);
}

int mlc_strncmp(const char *s1,const char *s2,size_t length) {
  while(length && *s1 != 0 && *s2 != 0) {
    if(*s1 != *s2)
      return( (*s2 - *s1) );

    length--;
    s1++;
    s2++;
  }

  if(!length || (*s1 == 0 && *s2 == 0))
    return(0);

  return(-1);
}

#define toupper(ch) ((((ch)>='a')&&((ch)<='z'))?((ch)+'A'-'a'):(ch))

int mlc_strncasecmp(const char *s1,const char *s2,size_t length) {
  while(length && *s1 != 0 && *s2 != 0) {
    if(toupper(*s1) != toupper(*s2))
      return( (toupper(*s2) - toupper(*s1)) );

    length--;
    s1++;
    s2++;
  }

  if(!length || (*s1 == 0 && *s2 == 0))
    return(0);

  return(-1);
}

int mlc_strcmp(const char *s1,const char *s2) {
	size_t i1,i2,max;

	i1 = mlc_strlen(s1);
	i2 = mlc_strlen(s2);
	if( i1 > i2 ) max = i1;
	else          max = i2;

	return( mlc_strncmp( s1,s2,max ) );
}

int mlc_strcasecmp(const char *s1,const char *s2) {
	size_t i1,i2,max;

	i1 = mlc_strlen(s1);
	i2 = mlc_strlen(s2);
	if( i1 > i2 ) max = i1;
	else          max = i2;

	return( mlc_strncasecmp( s1,s2,max ) );
}

size_t mlc_strlcpy(char *dest,const char *src,size_t space)
{
  size_t destlen = 0;
  if (--space > 0) {
    do {
      if ((*dest++ = *src++) == 0) return destlen;
      destlen++;
    } while (--space);
    *dest++ = 0;
  }
  return destlen;
}

size_t mlc_strlcat(char *dest,const char *src,size_t count)
{
  size_t len = mlc_strlen(dest);
  if (len >= count) return count;
  return len + mlc_strlcpy (dest + len, src, count - len);
}

/* gcc emits code that calls memcpy() */
#if !ONPC
void *memcpy (void *dest, const void *src, size_t n)
{
    return mlc_memcpy (dest, src, n);
}
#endif

void *mlc_memcpy(void *dest, const void *src, size_t n) {
  // rewrite by TT 31Mar06, taking odd dest into account

  if( ((uint32)src & 3) != ((uint32)dest & 3) ) {
    register uint8  *bsrc,*bdest;
    bsrc  = (uint8*)src;
    bdest = (uint8*)dest;

    // alignment is different - we can only do a bytewise copy
    while( n-- > 0 ) *bdest++ = *bsrc++;

  } else if (n>0) {
    size_t n4;
    register uint32 *dsrc,*ddest;
    uint8  *bsrc,*bdest;
    bsrc  = (uint8*)src;
    bdest = (uint8*)dest;

    // Do byte-copies until we hit an even 4 byte boundary
    while( ((uint32)bsrc & 3) && (n--> 0) ) *bdest++ = *bsrc++;

    // Do as many dword copies that we can
    dsrc  = (uint32*)bsrc;
    ddest = (uint32*)bdest;
    n4 = n / 4;
    while( n4-- ) *ddest++ = *dsrc++;

    // Copy the remaining bytes
    bsrc  = (uint8*)dsrc;
    bdest = (uint8*)ddest;
    n = n & 3;
    while( n-- ) *bdest++ = *bsrc++;
  }

  return (dest);
}

void *mlc_memset (void *dest, int c, size_t n)
{
    uint8 *d = dest;
    while (n--) *d++ = c;
    return dest;
}

char *mlc_strchr(const char *s,int c) {
  char *ret;

  ret = (char*)s;
  while( (*ret != c) && (*ret != 0) ) ret++;

  if( *ret == 0 ) return(NULL);

  return(ret);
}

char *mlc_strrchr(const char *s,int c) {
  char *ret;

  ret = (char*)s + mlc_strlen (s);
  while( (*ret != c) && (ret != s) ) ret--;

  if( ret == s ) return(NULL);

  return(ret);
}

void mlc_delay_ms (long time_in_ms) {
  #if defined (__arm__)
    // we only need the delay on the iPod, not when debugging on a PC
    if (time_in_ms > 10000) time_in_ms = 10000; // 10s should always be more than enough!
    long start = timer_get_current ();
    do {
      // pause
    } while (!timer_passed (start, time_in_ms*1000));
  #endif
}

void mlc_delay_us (long time_in_micro_s) {
  #if defined (__arm__)
    // we only need the delay on the iPod, not when debugging on a PC
    if (time_in_micro_s > 1000000) time_in_micro_s = 1000000; // 1s should always be more than enough!
    long start = timer_get_current ();
    do {
      // pause
    } while (!timer_passed (start, time_in_micro_s));
  #endif
}

long mlc_atoi (const char *str) {
  long n = 0, factor = 1;
  if (*str == '+') {
    str++;
  } else if (*str == '-') {
    *str++;
    factor = -1;
  }
  while (*str >= '0' && *str <= '9') {
      n = (n * 10) + *str++ - '0';
  }
  return n * factor;
}

uint16 mlc_atorgb (const char *str, uint16 dft) {
  if (*str == 'c') {
    // read 16bit rgb code
    str++;
    return mlc_atoi (str);
  } else if (*str == '(') {
    // read 3 values
    int v, r = 0, g = 0, b = 0;
    *str++;
    while (*str <= ' ') ++str; // skip whitespace
    while (*str >= '0' && *str <= '9') r = (r * 10) + *str++ - '0';
    if (*str == ',') ++str; // skip ,
    while (*str <= ' ') ++str; // skip whitespace
    while (*str >= '0' && *str <= '9') g = (g * 10) + *str++ - '0';
    if (*str == ',') ++str; // skip ,
    while (*str <= ' ') ++str; // skip whitespace
    while (*str >= '0' && *str <= '9') b = (b * 10) + *str++ - '0';
    if (*str == ')') {
      v = fb_rgb (r,g,b);
      //mlc_printf("r:%02x g:%02x b:%02x > %04x\n", r, g, b, v); // debug output
      return v;
    }
  }
  return dft;
}

void mlc_clear_screen () {
  // sets the cursor home and clears the screen
  printf_buflen = 0;
  #if !ONPC
    console_clear();
  #endif
}

void mlc_set_output_options (int buffered, int slow) {
  // buffered:
  //   pass <1> to have printf calls not output to console but store the text in a buffer
  //   pass <0> to have all buffered and new lines printed immediately
  // slow:
  //   pass a boolean for enabling to pause a little after each printf()
  #if !ONPC
    if (!printf_buffer) {
      printf_buffer = mlc_malloc (printf_buffer_size * sizeof (*printf_buffer));
    }
    if (!buffered && printf_buflen) {
      // flush the buffered data to console
      int i;
      console_suppress_fbupdate (1);
      for (i = 0; i < printf_buflen; i++) {
        print_to_console (printf_buffer[i], 0);
      }
      printf_buflen = 0;
      console_suppress_fbupdate (-1);
    }
    do_buffered_printf = buffered;
    do_slow_printf = slow && !buffered;
  #endif
}

// call this if you can still continue but want to make the user see what you just printed:
void mlc_show_critical_error () {
  mlc_set_output_options (0, 0);
  ipod_set_backlight (1);
  mlc_delay_ms (5000); // just pause for 5s
  keypad_flush ();
}

// call this if you can not continue, and want to make the user see what you just printed:
void mlc_show_fatal_error () {
  mlc_set_output_options (0, 0);
  mlc_printf ("\nHold Menu & %s to restart\n", ((ipod_get_hwinfo()->hw_rev < 0x40000) ? "Play" : "Select"));
  ipod_set_backlight (1);
  mlc_delay_ms (10000); // leave light on for 10s
  ipod_set_backlight (0);
  exit_irqs ();
  // wait for one minute, then put iPod to sleep
  for (int start = timer_get_current(); timer_passed (start, TIMER_MINUTE); ) {}
  pcf_standby_mode ();
}

void mlc_hexdump (void* addr, int len)
{
  int i;
  uint8 *p = (uint8 *)addr;
  for (i = 0; i < len; i+=8) {
    mlc_printf("%02x", (int)*p++);
    mlc_printf("%02x", (int)*p++);
    mlc_printf("%02x", (int)*p++);
    mlc_printf("%02x ", (int)*p++);
    mlc_printf("%02x", (int)*p++);
    mlc_printf("%02x", (int)*p++);
    mlc_printf("%02x", (int)*p++);
    mlc_printf("%02x\n", (int)*p++);
  }
}

void abort(void)
{
  for (;;) { }
}
