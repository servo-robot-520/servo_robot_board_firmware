/* STM32F411 memory layout (with bootloader) */
/* App starts at 0x0800_4000 (after 16KB bootloader) */
/* App region: 240KB (Sectors 1-5), OTA Temp: 128KB (Sector 6) */
MEMORY
{
    FLASH : ORIGIN = 0x08004000, LENGTH = 240K
    RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}

/* 堆栈配置 */
_stack_size = 0x2000; /* 8KB */
_heap_size = 0x1000;  /* 4KB */
