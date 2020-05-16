#include "bootloader.h"
#include "ipodhw.h"
#include "minilibc.h"
#include "console.h"
#include "menu.h"
#include "vfs.h"
#include "fb.h"

#include "config.h"

#define DEBUGPRINT(x) { mlc_printf (x); mlc_delay_ms (2000); }

static config_t config;

static const char * find_somewhere (const char **names, const char *what, int *fdOut)
{
    int fd = -1;
#if DEBUG
    mlc_printf (">> Looking for a %s...\n", what);
#endif
    for (; *names; names++) {
#if DEBUG
        mlc_printf (">> Trying |%s|...\n", *names);
#endif
        fd = vfs_open ((char *)*names);
        if (fd >= 0) break;
    }
#if DEBUG
    if (*names) {
        mlc_printf (">> Found it at %s.\n", *names);
    } else {
        mlc_printf (">>! Not found. :-(\n");
    }
#endif
    if (fdOut) *fdOut = fd;
    return *names;
}

const char *confnames[] = {
	// the following are for the music partition:
        "(hd0,1)/ipodloader.conf",
	"(hd0,1)/Notes/ipodloader.conf",
	"(hd0,1)/boot/ipodloader.conf",
	"(hd0,1)/loader.cfg",
	"(hd0,1)/Notes/loader.cfg",
	"(hd0,1)/boot/loader.cfg",
	// now, more of the same for people who haven't figured out file renaming
        "(hd0,1)/ipodloader.conf.txt",
	"(hd0,1)/Notes/ipodloader.conf.txt",
	"(hd0,1)/boot/ipodloader.conf.txt",
	"(hd0,1)/loader.cfg.txt",
	"(hd0,1)/Notes/loader.cfg.txt",
	"(hd0,1)/boot/loader.cfg.txt",
	// and some plain ones
        "(hd0,1)/ipodloader.txt",
        "(hd0,1)/Notes/ipodloader.txt",
        "(hd0,1)/boot/ipodloader.txt",
	"(hd0,1)/loader.txt",
	"(hd0,1)/Notes/loader.txt",
	"(hd0,1)/boot/loader.txt",

	// the following are for the ext2 partition (WinPods only):
	"(hd0,2)/ipodloader.conf",
	"(hd0,2)/boot/ipodloader.conf",
	"(hd0,2)/loader.cfg",
	"(hd0,2)/boot/loader.cfg",
	// and again...
	"(hd0,2)/ipodloader.conf.txt",
	"(hd0,2)/boot/ipodloader.conf.txt",
	"(hd0,2)/loader.cfg.txt",
	"(hd0,2)/boot/loader.cfg.txt",
	// and againer...
        "(hd0,2)/ipodloader.txt",
        "(hd0,2)/Notes/ipodloader.txt",
	"(hd0,2)/loader.txt",
	"(hd0,2)/Notes/loader.txt",

	// we can also read from the firmware partition:
	"(hd0,0)/lcnf",
	0 };

const char *kernnames[] = {
	// the following are for the music partition:
	"(hd0,1)/kernel.bin",
	"(hd0,1)/Notes/kernel.bin",
	"(hd0,1)/boot/kernel.bin",
	"(hd0,1)/linux.bin",
	"(hd0,1)/Notes/linux.bin",
	"(hd0,1)/boot/linux.bin",
	"(hd0,1)/vmlinux",
	"(hd0,1)/Notes/vmlinux",
	"(hd0,1)/boot/vmlinux",

	// the following are for the ext2 partition (WinPods only):
	"(hd0,2)/kernel.bin",
	"(hd0,2)/boot/kernel.bin",
	"(hd0,2)/linux.bin",
	"(hd0,2)/boot/linux.bin",
	"(hd0,2)/vmlinux",
	"(hd0,2)/boot/vmlinux",

	// we can also read the kernel from the firmware partition:
	"(hd0,0)/linx",
	0 };

int is_applefw_img (char *firstblock);

static config_image_t configimgs[MAX_MENU_ITEMS];

void config_init(void)
{
    char *configdata, *p;
    int fd, len, firstitem = 1;

    mlc_memset (&config, 0, sizeof (config));
    mlc_memset (&configimgs, 0, sizeof (configimgs));
    config.image = configimgs;
    config.timeout   = 15;
    config.def       = 1; // default item index in menu, 1-based
    config.backlight = 1;
    config.usegradient = 1;
    config.bgcolor   = fb_rgb(0,0,255);
    config.hicolor   = fb_rgb(64,128,0);
    config.beep_time = 50;
    config.beep_period = 30;

    {
        // preset default menu items
    
        int i = 0;
 
        config.image[i].title = "Apple OS";
        if (is_applefw_img ((void*)(ipod_get_hwinfo()->mem_base))) {
          config.image[i].type  = CONFIG_IMAGE_SPECIAL;
          config.image[i].path  = "ramimg";
        } else {
          config.image[i].type  = CONFIG_IMAGE_BINARY;
          config.image[i].path  = "(hd0,0)/aple";
          if (vfs_open (config.image[i].path) < 0) {
            config.image[i].path  = "(hd0,0)/osos";
          }
        }
        i++;

        config.image[i].type  = CONFIG_IMAGE_BINARY;
        config.image[i].title = "iPodLinux";
        config.image[i].path  = (char *)find_somewhere (kernnames, "kernel", NULL);
        if (config.image[i].path) { i++; }

        config.image[i].type  = CONFIG_IMAGE_ROCKBOX;
        config.image[i].title = "Rockbox";
        config.image[i].path  = "(hd0,1)/.rockbox/rockbox.ipod";
        if (vfs_open (config.image[i].path) >= 0) {
          i++;
        }

        config.image[i].type  = CONFIG_IMAGE_SPECIAL;
        config.image[i].title = "Disk Mode";
        config.image[i].path  = "diskmode";
        i++;

        config.image[i].type  = CONFIG_IMAGE_SPECIAL;
        config.image[i].title = "Sleep";
        config.image[i].path  = "standby";
        i++;

        config.items = i;
    }

    if (find_somewhere (confnames, "configuration file", &fd)) {

        // read the config file into the buffer at 'configdata'
        configdata = mlc_malloc (4096);
        mlc_memset (configdata, 0, 4096);
        if ((len = vfs_read (configdata, 1, 4096, fd)) == 4096) {
            mlc_printf ("Config file is too long, reading only first 4k\n");
            --len;
        }
        configdata[len] = 0;
        
        // change all CRs into LFs (for Windows and Mac users)
        p = configdata;
        while (*p) {
          if (*p == '\r') *p = '\n';
          ++p;
        }
        
        // now parse the file contents
        p = configdata;
        while (p && *p) {
            char *nextline = mlc_strchr (p, '\n');
            char *value, *keyend, *white;

            if (nextline) {
                *nextline = 0;
                do { nextline++; } while (*nextline == '\n'); // skips empty lines
            }

            while (*p == ' ' || *p == '\t') p++;

            if (*p == ';' || *p == '#') {
                // a comment line - ignore it
                value = 0;
            } else {
                if ((value = mlc_strchr (p, '@')) != 0 || (value = mlc_strchr (p, '=')) != 0 || (value = mlc_strchr (p, ' ')) != 0) {
                    *value = 0;
                    do value++; while (*value == ' ' || *value == '\t' || *value == '=' || *value == '@');
                }
            }

            if (!value) {
                p = nextline;
                continue;
            }

            keyend = p;
            while (*keyend) keyend++; // goes to the NUL
            do keyend--; while (*keyend == ' ' || *keyend == '\t'); // goes to last nonblank
            *++keyend = 0; // first blank at end becomes end of string

            if (!mlc_strcmp (p, "default")) {
                config.def = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "timeout")) {
                config.timeout = 0;
                config.timeout = mlc_atoi (value);
                if (config.timeout != 0 && config.timeout < 2) config.timeout = 2; // less than 2s is quite unusable
            } else if (!mlc_strcmp (p, "debug")) {
                config.debug = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "backlight")) {
                config.backlight = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "contrast")) {
                config.contrast = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "bg_gradient")) {
                config.usegradient = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "bg_color")) {
                config.bgcolor = mlc_atorgb (value, config.bgcolor);
            } else if (!mlc_strcmp (p, "hilight_color")) {
                config.hicolor = mlc_atorgb (value, config.hicolor);
            } else if (!mlc_strcmp (p, "beep_duration")) {
                config.beep_time = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "beep_period")) {
                config.beep_period = mlc_atoi (value);
            } else if (!mlc_strcmp (p, "ata_standby_code")) {
                config.ata_standby_code = mlc_atoi (value);
            } else {
                // it's a menu item
                if (firstitem) {
                  // the user wants to list the files explicitly - discard the default list
                  firstitem = 0;
                  config.items = 0;
                }
                while(	   *(white = (value + mlc_strlen(value) - 1)) == ' '
			|| *(white = (value + mlc_strlen(value) - 1)) == '\t')
				*white = '\0'; // Remove trailing whitespace
                config.image[config.items].type  = CONFIG_IMAGE_BINARY;
                config.image[config.items].title = p;
                config.image[config.items].path  = value;
                if (!mlc_strncasecmp (value, "rb:", 3)) {
                    config.image[config.items].path += 3;
                    config.image[config.items].type = CONFIG_IMAGE_ROCKBOX;
                } else if (value[0] != '(' && value[0] != '[') { // no partition specifier -> it's not a path but command
                    config.image[config.items].type = CONFIG_IMAGE_SPECIAL;
                }
                config.items++;
                if (config.items >= MAX_MENU_ITEMS) {
                    break;
                }
            }

            p = nextline;
        }
    }

    // finally, some sanity checks:
    if (config.def > config.items) config.def = config.items;
}

config_t *config_get(void) {
  return &config;
}
