.text
.global _start

_start:
    // --- MOVZ/MOVK/MOVN with legal shifts ---
    movz    x10, #0xabcd
    movz    x11, #1, lsl #16
    movk    x12, #0x7e57
    movk    x13, #0xf0f0, lsl #16
    movn    x14, #0
    movn    x15, #0xffff, lsl #16

    // --- Zero/sign extend ---
    mov     w1, #0x1234
    mov     x16, x1          // use x16 for print
    mov     w2, #-42
    sxtw    x17, w2
    mov     w3, #0x22
    sxth    x18, w3

    // --- Memory ops with shift/extend ---
    mov     x20, #0x1000
    mov     x21, #0x5
    str     x21, [x20]
    mov     x22, #8
    str     x21, [x20, x22, lsl #3]
    ldr     x23, [x20]
    ldr     x24, [x20, x22, lsl #3]
    mov     w4, #8
    str     x21, [x20, w4, uxtw #3]
    ldr     x25, [x20, w4, uxtw #3]

    // --- Print results using msg buffer ---
    ldr     x1, =msg_10
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_11
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_12
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_13
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_14
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0
    
    ldr     x1, =msg_15
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_16
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_17
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    ldr     x1, =msg_18
    mov     x2, #8
    mov     x0, #1
    mov     x8, #64
    svc     #0

    mov     x8, #93
    mov     x0, #0
    svc     #0

.data
msg_10: .asciz "x10 done\n"
msg_11: .asciz "x11 done\n"
msg_12: .asciz "x12 done\n"
msg_13: .asciz "x13 done\n"
msg_14: .asciz "x14 done\n"
msg_15: .asciz "x15 done\n"
msg_16: .asciz "x16 done\n"
msg_17: .asciz "x17 done\n"
msg_18: .asciz "x18 done\n"
