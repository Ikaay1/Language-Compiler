section .text
global our_code_starts_here
our_code_starts_here:
  mov rax, 5
  mov [rsp + -16], rax
  mov rax, [rsp + -16]
  mov [rsp + -24], rax
  mov rax, [rsp + -16]
  add rax, [rsp + -24]
  ret
