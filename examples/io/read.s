.text
.global _start

_start:
    // Set up read(fd=0, buf=buffer, count=5)
    mov x0, #0            // fd = stdin
    adr x1, buffer        // x1 = address of buffer
    mov x2, #5            // count = 5
    mov x8, #63           // SYS_READ
    svc #0

    // Set up write(fd=1, buf=buffer, count=5)
    mov x0, #1            // fd = stdout
    adr x1, buffer        // x1 = address of buffer
    mov x2, #5
    mov x8, #64           // SYS_WRITE
    svc #0

    // exit(0)
    mov x0, #0
    mov x8, #93
    svc #0

.data
buffer:
.skip 5               // Allocate 5 bytes as buffer

