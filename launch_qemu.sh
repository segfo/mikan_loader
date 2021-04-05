#!/bin/sh
echo $1
mkdir -p mnt/EFI/BOOT
cp $1 mnt/EFI/BOOT/BOOTx64.EFI
qemu-system-x86_64 -m 2G -monitor stdio --bios firmware/ovmf.fd -drive format=raw,file=fat:rw:mnt -vga std