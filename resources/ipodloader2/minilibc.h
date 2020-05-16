#ifndef _MINILIBC_H_
#define _MINILIBC_H_

#include "bootloader.h"

/* Assume: width of stack == width of int. Don't use sizeof(char *) or
other pointer because sizeof(char *)==4 for LARGE-model 16-bit code.
Assume: width is a power of 2 */
#define	STACK_WIDTH	sizeof(int)

/* Round up object width so it's an even multiple of STACK_WIDTH.
Using & for division here, so STACK_WIDTH must be a power of 2. */
#define	TYPE_WIDTH(TYPE)				\
	((sizeof(TYPE) + STACK_WIDTH - 1) & ~(STACK_WIDTH - 1))


/* point the va_list pointer to LASTARG,
then advance beyond it to the first variable arg */
#define	mlc_va_start(PTR, LASTARG)				\
	PTR = (mlc_va_list)((char *)&(LASTARG) + TYPE_WIDTH(LASTARG))

#define mlc_va_end(PTR)	/* nothing */

/* Increment the va_list pointer, then return
(evaluate to, actually) the previous value of the pointer.
WHEEE! At last; a valid use for the C comma operator! */
#define mlc_va_arg(PTR, TYPE)	(			\
	PTR = (uint8*)PTR + TYPE_WIDTH(TYPE)		\
				,			\
	*((TYPE *)((char *)(PTR) - TYPE_WIDTH(TYPE)))	\
				)
/* Every other compiler/libc seems to be using 'void *', so...
(I _was_ using 'unsigned char *') */
typedef void *mlc_va_list;

int mlc_sprintf(char *buf, const char *fmt, ...);
int mlc_vprintf(const char *fmt, mlc_va_list args);
int mlc_printf(const char *fmt, ...);

void   mlc_malloc_init(void);
void  *mlc_malloc(size_t num);
size_t mlc_strlen(const char *);
int    mlc_strcmp(const char *s1,const char *s2);
int    mlc_strcasecmp(const char *s1,const char *s2);
int    mlc_strncmp(const char *s1,const char *s2,size_t maxlen);
int    mlc_strncasecmp (const char *s1,const char *s2,size_t maxlen);
size_t mlc_strlcpy(char *dest,const char *src,size_t count);
size_t mlc_strlcat(char *dest,const char *src,size_t count);
void  *mlc_memcpy(void *dest,const void *src,size_t n);
void  *mlc_memset(void *dest,int c,size_t n);
char  *mlc_strchr(const char *s,int c);
char  *mlc_strrchr(const char *s,int c);
int    mlc_memcmp(const void *sv1,const void *sv2,size_t length);
void   mlc_delay_ms (long time_in_ms);
void   mlc_delay_us (long time_in_micro_s);
long   mlc_atoi (const char *str);
uint16 mlc_atorgb (const char *str, uint16 dft);
void   mlc_set_output_options (int buffered, int slow);
void   mlc_show_critical_error (); // call this if you can still continue but want to make the user see what you just printed
void   mlc_show_fatal_error (); // call this if you can not continue, and want to make the user see what you just printed
void   mlc_clear_screen ();
void   mlc_hexdump (void* addr, int len);

#endif
