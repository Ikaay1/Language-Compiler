section .text
global our_code_starts_here
our_code_starts_here:
  mov rax, 2
  mov [rsp + -16], rax
  mov rax, 3
  imul rax, [rsp + -16]
  mov [rsp + -16], rax
  mov rax, 3
  add rax, [rsp + -16]
  ret
