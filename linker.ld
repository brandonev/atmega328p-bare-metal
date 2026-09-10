ENTRY(__vectors)

MEMORY
{
    flash (rx) : ORIGIN = 0x000000, LENGTH = 32K
    sram  (rw) : ORIGIN = 0x800100, LENGTH = 2K
}

SECTIONS
{
    .text :
    {
        KEEP(*(.vectors))
        *(.text.reset)
        *(.text*)
        *(.rodata*)
    } > flash

    .data :
    {
        __data_start = .;

        *(.data)
        *(.data.*)

        __data_end = .;
    } > sram AT > flash

    __data_load_start = LOADADDR(.data);

    .bss (NOLOAD) :
    {
        __bss_start = .;

        *(.bss)
        *(.bss.*)
        *(COMMON)

        __bss_end = .;
    } > sram
}
