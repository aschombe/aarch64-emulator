.text
.global _start

_start:
    // openat(AT_FDCWD = -100, "file1.txt", O_CREAT|O_WRONLY|O_TRUNC, 0644)
    mov x0, -100
    adr x1, fname
    mov x2, 577
    mov x3, 420
    mov x8, 56
    svc 0
    mov x20, x0

    // Compute length of msg1
    adr x1, msg1
    mov x2, 0

len_loop1:
    ldrb w3, [x1, x2]
    cbz w3, len_done1
    add x2, x2, 1
    b len_loop1

len_done1:
    // write(fd, msg1, length)
    mov x0, x20
    adr x1, msg1
    mov x8, 64
    svc 0

    // close(fd)
    mov x0, x20
    mov x8, 57
    svc 0

    // exit(0)
    mov x0, 0
    mov x8, 93
    svc 0

.data
fname: .asciiz "file1.txt"
msg1: .asciiz "Dynamic length write test!\n"
