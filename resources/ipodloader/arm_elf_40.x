
OUTPUT_FORMAT("elf32-littlearm", "elf32-bigarm",
	      "elf32-littlearm")
OUTPUT_ARCH(arm)
ENTRY(_start)

SECTIONS
{
  . = 0x40000000;

  .text : { *(.text) }

    __data_start__ = . ;
  .data : { *(.data) }

  __bss_start__ = .;
  .bss : {
     *(.bss);
     __bss_end__ = . ;
   }
}

