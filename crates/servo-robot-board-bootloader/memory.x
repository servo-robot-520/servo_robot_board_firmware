/* Bootloader linker script for STM32F411CEU6 */
/* Bootloader occupies Sector 0 (16KB) at 0x0800_0000 */

MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 16K
    RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

_stack_size = 0x1000; /* 4KB stack for bootloader */
