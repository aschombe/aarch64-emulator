.text
.global _start

_start:
    adr x0, arr
    ldr x1, [x0]

    mov x0, x1
    mov x8, 93
    svc 0

.data
arr1:
    .rept 5
    .quad 10
    .endr

