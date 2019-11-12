
#ifndef TOOLS_H
#define TOOLS_H

#define inl(a) (*(volatile unsigned long *) (a))
#define outl(a,b) (*(volatile unsigned long *) (b) = (a))
#define inb(a) (*(volatile unsigned char *) (a))
#define outb(a,b) (*(volatile unsigned char *) (b) = (a))

/* find out which ipod revision we're running on */
void get_ipod_rev();

/* get current usec counter */
int timer_get_current();

/* check if number of seconds has past */
int timer_check(int clock_start, int usecs);

/* wait for r0 useconds */
int wait_usec(int usecs);

/* wait for LCD with timeout */
void lcd_wait_write();

/* send LCD data */
void lcd_send_data(int data_lo, int data_hi);

/* send LCD command */
void lcd_prepare_cmd(int cmd);

/* send LCD command and data */
void lcd_cmd_and_data(int cmd, int data_lo, int data_hi);

typedef struct _img {
	unsigned short offy;		// #0
	unsigned short offx;		// #2
	unsigned short height;		// #4
	unsigned short width;		// #6
	unsigned short data_width;	// #8
	unsigned short img_type;	// #10
	unsigned long pad0;		// #12
	unsigned long len;		// #16
	unsigned char *data;		// #20
} img;

void display_image(img *img, int draw_bg);

int opto_keypad_read();

#endif
