.global interrupt_handlers

.macro make_interrupt_handler isr error

.global interrupt_handler_\isr
interrupt_handler_\isr:

.if \error == 0
    push 0
    .endif

    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax
    push rbp

    push rax

    mov rdi, \isr
    mov rsi, rsp

    ; call the generic interrupt handler
    call generic_interrupt_handler

    ; pop the error code
    add rsp, 8

    mov rsp, rdi
    pop rbp
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    iretq
.endm

.macro interrupt_handler_no_error_code isr
    make_interrupt_handler \isr, 0
.endm

.macro interrupt_handler_error_code isr
    make_interrupt_handler \isr, 1
.endm

interrupt_handler_no_error_code 0
interrupt_handler_no_error_code 1
interrupt_handler_no_error_code 2
interrupt_handler_no_error_code 3
interrupt_handler_no_error_code 4
interrupt_handler_no_error_code 5
interrupt_handler_no_error_code 6
interrupt_handler_no_error_code 7

interrupt_handler_error_code 8
interrupt_handler_error_code 10
interrupt_handler_error_code 11
interrupt_handler_error_code 12
interrupt_handler_error_code 13
interrupt_handler_error_code 14

interrupt_handler_no_error_code 16

interrupt_handler_error_code 17

interrupt_handler_no_error_code 18
interrupt_handler_no_error_code 19
interrupt_handler_no_error_code 20

interrupt_handler_error_code 30

%assign i 32
%rep 224
    interrupt_handler_no_error_code i
%assign i i + 1
%endrep

section .rodata

interrupt_handlers:
    .quad interrupt_handler_0
    .quad interrupt_handler_1
    .quad interrupt_handler_2
    .quad interrupt_handler_3
    .quad interrupt_handler_4
    .quad interrupt_handler_5
    .quad interrupt_handler_6
    .quad interrupt_handler_7
    .quad interrupt_handler_8
    .quad 0
    .quad interrupt_handler_10
    .quad interrupt_handler_11
    .quad interrupt_handler_12
    .quad interrupt_handler_13
    .quad interrupt_handler_14
    .quad 0
    .quad interrupt_handler_16
    .quad interrupt_handler_17
    .quad interrupt_handler_18
    .quad interrupt_handler_19
    .quad interrupt_handler_20
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad 0
    .quad interrupt_handler_30
    .quad 0
%assign i 32
%rep 224
    .quad interrupt_handler_%+i
%assign i i + 1
%endrep