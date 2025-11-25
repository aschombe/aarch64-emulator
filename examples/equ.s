// This is an example to demonstrate the equ directive in assembly language.

.text
.globl _start

_start:
    mov x0, #num
    mov x8, #93          // syscall: exit
    svc 0

.data
.equ num, 42
