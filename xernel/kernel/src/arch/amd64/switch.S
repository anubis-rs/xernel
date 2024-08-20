.global switch_context

switch_context:
    sub rsp, 0x30
    mov [rsp + 0x00], rbx
    mov [rsp + 0x08], rbp
    mov [rsp + 0x10], r12
    mov [rsp + 0x18], r13
    mov [rsp + 0x20], r14
    mov [rsp + 0x28], r15
    mov [rdi], rsp

    mov rsp, rsi
    mov rbx, [rsp + 0x00]
    mov rbp, [rsp + 0x08]
    mov r12, [rsp + 0x10]
    mov r13, [rsp + 0x18]
    mov r14, [rsp + 0x20]
    mov r15, [rsp + 0x28]
    add rsp, 0x30
    ret