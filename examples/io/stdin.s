.text
.global _start

_start:
    // read(0, buffer, 16)
    mov x0, 0
    adr x1, buffer
    mov x2, 16
    mov x8, 63
    svc 0
    mov x20, x0

    // write(1, buffer, bytes_read)
    mov x0, 1
    adr x1, buffer
    mov x2, x20
    mov x8, 64
    svc 0

    // exit(0)
    mov x0, 0
    mov x8, 93
    svc 0

.data
buffer: .skip 64

