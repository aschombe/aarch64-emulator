.text
.global _start

_start:
    // === Test MOVZ (move with zero) ===
    movz    x0, #0xABCD         // x0 = 0x000000000000ABCD
    bl      print_result

    movz    x1, #0x1234, lsl #16 // x1 = 0x0000000012340000
    bl      print_result

    movz    x2, #0x5678, lsl #32 // x2 = 0x0000567800000000
    bl      print_result

    movz    x3, #0x9ABC, lsl #48 // x3 = 0x9ABC000000000000
    bl      print_result

    // === Test MOVK (move with keep) builds up 0x123456789ABCF0F0 in x4 ===
    movz    x4, #0xF0F0           // x4 = 0x000000000000F0F0
    movk    x4, #0x9ABC, lsl #48  // x4 = 0x9ABC00000000F0F0
    movk    x4, #0x5678, lsl #32  // x4 = 0x9ABC56780000F0F0
    movk    x4, #0x1234, lsl #16  // x4 = 0x9ABC56781234F0F0
    movk    x4, #0x0000, lsl #0   // (no change—shows keep when zero)
    bl      print_result

    // === Test MOVN (move with NOT, optional) ===
    movn    x5, #0xFFFF           // x5 = 0xFFFFFFFFFFFF0000 (all ones except lower 16 as zeros)
    bl      print_result

    movn    x6, #0x1234, lsl #16  // x6 = ~(0x1234 << 16), i.e. all ones except 000123400 as zeros
    bl      print_result

    // Exit
    mov     x8, #93
    mov     x0, #0
    svc     0

// Prints signed (decimal) value in xN
print_result:
    sub     sp, sp, #32
    mov     x20, sp
    mov     x21, x0

    cmp     x21, #0
    b.ne    check_sign
    ldr     x0, =zerochar
    mov     x1, x0
    mov     x2, #1
    mov     x8, #64
    mov     x0, #1
    svc     0
    b       print_nl

check_sign:
    cmp     x21, #0
    b.ge    convert
    ldr     x0, =minuschar
    mov     x1, x0
    mov     x2, #1
    mov     x8, #64
    mov     x0, #1
    svc     0
    sub     x21, xzr, x21     // Use sub, not neg

convert:
    mov     x22, x21
    mov     x23, #0

convert_loop:
    mov     x24, #10
    udiv    x25, x22, x24
    mul     x26, x25, x24
    sub     x26, x22, x26
    add     x26, x26, 48
    strb    w26, [x20, x23]
    add     x23, x23, #1
    mov     x22, x25
    cbnz    x22, convert_loop

print_loop:
    subs    x23, x23, #1
    blt     print_nl
    ldrb    w27, [x20, x23]
    strb    w27, [x20, #0]
    mov     x0, #1
    mov     x1, x20
    mov     x2, #1
    mov     x8, #64
    svc     0
    b       print_loop

print_nl:
    ldr     x0, =newline
    mov     x1, x0
    mov     x2, #1
    mov     x8, #64
    mov     x0, #1
    svc     0
    add     sp, sp, #32
    ret

.data
newline: .asciz "\n"
minuschar: .asciz "-"
zerochar: .asciz "0"

