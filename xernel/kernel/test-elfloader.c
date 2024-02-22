
char *message = "hello from userspace";

int syscall(long syscall_number, char *arg) {
    int ret;

    asm volatile(
        "syscall"
        : "=a" (ret)
        : "a" (syscall_number), "D" (arg)
    );

    return ret;
}

void _start() {
    while (1) {
        for (int i = 0; i < 1000000; i++) {
            asm volatile("nop");
        }

        syscall(5, message);
    }
}
