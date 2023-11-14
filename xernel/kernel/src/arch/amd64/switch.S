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
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    mov [rdi], rsp
    ret

restore_context:
    mov rsp, rdi

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbx

    ret
