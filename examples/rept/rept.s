.text
.global _start

_start:
    mov x0, 1              // File descriptor: stdout
    adr x1, reptmsg        // Address of buffer (use 'adr', not 'ldr =')
    mov x2, 5              // Number of bytes to write
    mov x8, 64             // Syscall number for write
    svc 0

    mov x0, 0              // exit code 0
    mov x8, 93             // Syscall number for exit
    svc 0

.data
reptmsg:
.byte 65
.byte 65
.byte 65
.byte 65
.byte 65
