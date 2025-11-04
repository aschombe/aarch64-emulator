.text
.global _start

_start:
    // openat(AT_FDCWD, "file1.txt", O_RDONLY)
    mov x0, -100
    adr x1, fname
    mov x2, 0
    mov x3, 0
    mov x8, 56
    svc 0
    mov x19, x0

    // read(fd, buffer, 64)
    mov x0, x19
    adr x1, buffer
    mov x2, 64
    mov x8, 63
    svc 0
    mov x21, x0

    // write(1, buffer, bytes_read)
    mov x0, 1
    adr x1, buffer
    mov x2, x21
    mov x8, 64
    svc 0

    // close(fd)
    mov x0, x19
    mov x8, 57
    svc 0

    // exit(0)
    mov x0, 0
    mov x8, 93
    svc 0

.data
fname: .asciiz "file1.txt"
buffer: .skip 64

