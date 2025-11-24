.text
.global _start

_start:
    .rept 5
    // Output char 'A'
    mov x0, 1          // File descriptor stdout
    adr x1, char       // Address of char
    mov x2, 1          // Length
    mov x8, 64         // syscall write
    svc 0
    .endr

    // exit(0)
    mov x8, 93
    mov x0, 0
    svc 0

.data
char:   .byte 'A'

