
section .text
global our_code_starts_here
our_code_starts_here:
  mov rax, 12
neg rax
sub rax, 1
  ret
