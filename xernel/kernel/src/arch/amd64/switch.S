.global switch_context
.global save_context
.global restore_context

switch_context:
  push rbp
  push rbx
  push r12
  push r13
  push r14
  push r15

  mov [rdi], rsp
  mov rsp, rsi

  pop r15
  pop r14
  pop r13
  pop r12
  pop rbx
  pop rbp

  ret

save_context:
    mov [rdi], r15
    mov [rdi + 8], r14
    mov [rdi + 16], r13
    mov [rdi + 24], r12
    mov [rdi + 32], rbx
    mov [rdi + 40], rbp

    # rsp???
    ret

restore_context:
    mov rsp, rdi

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp

    ret
