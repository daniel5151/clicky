
OUTPUT_FORMAT("elf32-littlearm", "elf32-bigarm",
	      "elf32-littlearm")
OUTPUT_ARCH(arm)
ENTRY(_start)

SECTIONS
{
  . = 0x40000000;

  .text : { *(.text) }

  __data_start__ = . ;
  .data : { *(.data) *(.rodata) }

  __bss_start__ = .;
  .bss : {
     *(.bss);
     __bss_end__ = . ;
   }

  __exidx_start = .;
  .ARM.exidx   : { *(.ARM.exidx* .gnu.linkonce.armexidx.*) }
  __exidx_end = .;
}

