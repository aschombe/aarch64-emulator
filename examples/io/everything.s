.text
.global _start

_start:
    // Open/create file for O_WRONLY|O_CREAT (flags = 65)
    mov x8, 56                  // SYS_openat
    mov x0, -100                // AT_FDCWD
    ldr x1, =fname
    mov x2, 65                  // O_WRONLY | O_CREAT
    mov x3, 0o644
    svc 0
    mov x20, x0                 // Save fd

    // Write 12 bytes (length of "HelloWorld!\n")
    ldr x1, =writemsg
    mov x2, 12
    mov x8, 64                  // SYS_write
    mov x0, x20
    svc 0

    // Lseek to start
    mov x8, 62
    mov x0, x20
    mov x1, 0
    mov x2, 0                   // SEEK_SET
    svc 0

    // Read back to buffer
    ldr x1, =readbuf
    mov x2, 12
    mov x8, 63
    mov x0, x20
    svc 0

    // Print read buffer
    mov x0, 1
    ldr x1, =readbuf
    mov x2, 12
    mov x8, 64
    svc 0

    // Lseek to offset 5 (truncate by overwrite, no real ftruncate syscall)
    mov x8, 62
    mov x0, x20
    mov x1, 5
    mov x2, 0                   // SEEK_SET
    svc 0

    // Write over with "ABCD"
    ldr x1, =truncmsg
    mov x2, 4
    mov x8, 64
    mov x0, x20
    svc 0

    // Lseek to 0, read 12 again
    mov x8, 62
    mov x0, x20
    mov x1, 0
    mov x2, 0
    svc 0

    ldr x1, =readbuf
    mov x2, 12
    mov x8, 63
    mov x0, x20
    svc 0

    mov x0, 1
    ldr x1, =readbuf
    mov x2, 12
    mov x8, 64
    svc 0

    // Close fd
    mov x8, 57
    mov x0, x20
    svc 0

    // Re-open for O_RDONLY (flags=0)
    mov x8, 56
    mov x0, -100
    ldr x1, =fname
    mov x2, 0
    mov x3, 0
    svc 0
    mov x21, x0

    // Try to write to read-only fd
    ldr x1, =failmsg
    mov x2, 11
    mov x8, 64
    mov x0, x21
    svc 0

    // O_WRONLY | O_APPEND = 4096 + 1 = 4097
    mov x8, 56
    mov x0, -100
    ldr x1, =fname
    mov x2, 4097
    mov x3, 0
    svc 0
    mov x22, x0

    ldr x1, =appmsg
    mov x2, 7
    mov x8, 64
    mov x0, x22
    svc 0

    // Close append fd
    mov x8, 57
    mov x0, x22
    svc 0

    // Exit
    mov x0, 0
    mov x8, 93
    svc 0

.data
fname:      .asciz "vfstest.txt"
writemsg:   .asciz "HelloWorld!\n"
truncmsg:   .asciz "ABCD"
failmsg:    .asciz "should_fail"
appmsg:     .asciz "APPEND!"
readbuf:    .skip 64

