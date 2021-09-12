#!/bin/sh
echo $1
cd ../mikan_kernel/
cargo build
cd ../mikan_loader/
mkdir -p mnt/EFI/BOOT
cp $1 mnt/EFI/BOOT/BOOTx64.EFI
cp ../mikan_kernel/target/x86_64-unknown-none/debug/mikan_kernel ./mnt/
qemu-system-x86_64 -S -gdb tcp::1234 -m 2G -monitor stdio --bios firmware/ovmf.fd -drive format=raw,file=fat:rw:mnt -vga std 
# qemu-system-x86_64 -m 2G -monitor stdio --bios firmware/ovmf.fd -drive format=raw,file=fat:rw:mnt -vga std