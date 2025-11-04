.text
.global _start

_start:
    // openat(AT_FDCWD, "append.txt", O_CREAT|O_RDWR, 0644)
    mov x0, -100
    adr x1, fname
    mov x2, 66
    mov x3, 420
    mov x8, 56
    svc 0
    mov x19, x0

    // compute length of msg1
    adr x1, msg1
    mov x2, 0

len_loop2:
    ldrb w3, [x1, x2]
    cbz w3, len_done2
    add x2, x2, 1
    b len_loop2

len_done2:
    // write(fd, msg1, len)
    mov x0, x19
    adr x1, msg1
    mov x8, 64
    svc 0

    // lseek(fd, 0, SEEK_END)
    mov x0, x19
    mov x1, 0
    mov x2, 2
    mov x8, 62
    svc 0

    // compute length of msg2
    adr x1, msg2
    mov x2, 0

len_loop3:
    ldrb w3, [x1, x2]
    cbz w3, len_done3
    add x2, x2, 1
    b len_loop3

len_done3:
    // write(fd, msg2, len)
    mov x0, x19
    adr x1, msg2
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
fname: .asciiz "append.txt"
msg1: .asciiz "This is the first part.\n"
msg2: .asciiz "This is appended via lseek.\n"
