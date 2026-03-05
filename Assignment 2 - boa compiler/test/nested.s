section .text
global our_code_starts_here
our_code_starts_here:
  mov rax, 5
  mov [rsp + -16], rax
  mov rax, [rsp + -16]
  mov [rsp + -24], rax
  mov rax, 5
  add rax, [rsp + -24]
  mov [rsp + -24], rax
  mov rax, [rsp + -16]
  mov [rsp + -32], rax
  mov rax, [rsp + -24]
  mov [rsp + -40], rax
  mov rax, 2
  imul rax, [rsp + -40]
  add rax, [rsp + -32]
  ret
