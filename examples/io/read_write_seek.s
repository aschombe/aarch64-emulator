.text
.global _start

_start:
    // openat(AT_FDCWD, "roundtrip.txt", O_CREAT|O_RDWR|O_TRUNC, 0644)
    mov x0, -100
    adr x1, fname
    mov x2, 578
    mov x3, 420
    mov x8, 56
    svc 0
    mov x19, x0

    // compute message length
    adr x1, msg
    mov x2, 0

len_loop4:
    ldrb w3, [x1, x2]
    cbz w3, len_done4
    add x2, x2, 1
    b len_loop4

len_done4:
    // write(fd, msg, len)
    mov x0, x19
    adr x1, msg
    mov x8, 64
    svc 0

    // seek to start
    mov x0, x19
    mov x1, 0
    mov x2, 0
    mov x8, 62
    svc 0

    // read(fd, buffer, 64)
    mov x0, x19
    adr x1, buffer
    mov x2, 64
    mov x8, 63
    svc 0
    mov x21, x0

    // write to stdout
    mov x0, 1
    adr x1, buffer
    mov x2, x21
    mov x8, 64
    svc 0

    // close and exit
    mov x0, x19
    mov x8, 57
    svc 0

    mov x0, 0
    mov x8, 93
    svc 0

.data
fname: .asciiz "roundtrip.txt"
msg: .asciiz "Roundtrip dynamic test successful!\n"
buffer: .skip 64

