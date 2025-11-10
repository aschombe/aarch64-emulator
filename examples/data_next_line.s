.text
.global _start

_start:
    mov x0, 1
    adr x1, msg      // Load address of the message into x0
    adr x2, len      // Load address of the length into x1
    ldr w2, [x2]    // Load length of the message into x
    mov x8, 64      // syscall: write
    svc 0           // Make syscall

.data
msg: .asciz "Hello, World!\n"
len: .word 14
