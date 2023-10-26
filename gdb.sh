#!/bin/bash

# Call gdb and connect to qemu
gdb target/x86_64/debug/xernel \
    -ex "target remote :1234" \
    -ex "break kernel_main" \
    -ex "continue"
