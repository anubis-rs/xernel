# builds and runs the barebones kernel in qemu

set -x -e

# 1. build the kernel
cargo build

# 2. fetch and build limine
if [ ! -d "limine" ]; then
    git clone https://github.com/limine-bootloader/limine.git --branch=v3.0-branch-binary --depth=1
    make -C limine
fi

# 3. build the iso file
rm -rf iso_root
mkdir -p iso_root
cp target/x86_64/debug/xernel limine.cfg limine/limine.sys limine/limine-cd.bin limine/limine-cd-efi.bin iso_root/
xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-cd-efi.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o xernel.iso
limine/limine-deploy xernel.iso
rm -rf iso_root

# 4. run the kernel with UEFI
qemu-system-x86_64 -bios ./uefi-edk2/OVMF.fd -cdrom xernel.iso --no-reboot -d int -D qemulog.log
