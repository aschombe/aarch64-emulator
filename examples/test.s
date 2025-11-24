.text
.global _start

_start:
    mov x0, 5
    mov x1, 10
    add x2, x0, 0

    mov x0, 0
    mov x8, 93
    svc 0
