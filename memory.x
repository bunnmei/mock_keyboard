/* memory.x for nice!nano with UF2 bootloader and SoftDevice S140 v6.1.1 */

MEMORY
{
    FLASH : ORIGIN = 0x00000000 + 156K, LENGTH = 1024K - 156K
    RAM : ORIGIN = 0x20000000 + 31K, LENGTH = 256K - 31K
}