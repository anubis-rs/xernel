# builds and runs the barebones kernel in qemu

set -x -e

DISKNAME="xernel.hdd"
DISKSIZE=64

cd xernel/kernel

# 1. build the kernel
cargo build

# 2. fetch and build limine
if [ ! -d "limine" ]; then
    git clone https://github.com/limine-bootloader/limine.git --branch=v3.0-branch-binary --depth=1
    make -C limine
fi

dd if=/dev/zero of=$DISKNAME bs=1M count=0 seek=$DISKSIZE

mformat -i $DISKNAME -F
mcopy -i $DISKNAME ../../target/x86_64/debug/xernel ::/xernel
mcopy -i $DISKNAME limine.cfg ::/limine.cfg
mmd -i $DISKNAME ::/EFI
mmd -i $DISKNAME ::/EFI/BOOT
mcopy -i $DISKNAME limine/BOOTX64.EFI ::/EFI/BOOT

# 4. run the kernel with UEFI
qemu-system-x86_64 -bios ./uefi-edk2/OVMF.fd -cdrom $DISKNAME --no-reboot --no-shutdown -d int -D qemulog.log
