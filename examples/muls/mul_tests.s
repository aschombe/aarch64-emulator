.text
.global _start

_start:
    // UMULL: Unsigned 32 x 32 -> 64
    mov     w0, #100
    mov     w1, #5
    umull   x2, w0, w1    // x2 = 100 * 5 = 500

    // SMULL: Signed 32 x 32 -> 64
    mov     w3, #-7
    mov     w4, #8
    smull   x5, w3, w4    // x5 = -7 * 8 = -56

    // UMLAL: Unsigned Multiply-Accumulate Long (emulated)
    mov     w7, #4
    mov     w8, #3
    umull   x19, w7, w8   // x19 = 4 * 3 = 12
    mov     x6, #20
    add     x6, x6, x19   // x6 = 32

    // SMLAL: Signed Multiply-Accumulate Long (emulated)
    mov     w10, #-7
    mov     w11, #2
    smull   x20, w10, w11 // x20 = -14
    mov     x9, #-5
    add     x9, x9, x20   // x9 = -19

    // UMULH: Unsigned high 64 bits
    mov     x12, #1000        // Instead of 1000000000
    mov     x13, #1000
    umulh   x14, x12, x13

    mov     x15, #-1000
    mov     x16, #2000
    smulh   x17, x15, x16

    // dump results, one per line
    mov    x0, x2
    bl     dump_result
    mov    x0, x5
    bl     dump_result
    mov    x0, x6
    bl     dump_result
    mov    x0, x9
    bl     dump_result
    mov    x0, x14
    bl     dump_result
    mov    x0, x17
    bl     dump_result

    mov     x8, #93       // syscall: exit
    mov     x0, #0        // status: 0
    svc     0

dump_result:
    //convert signed x0 to decimal string;print with newline

    sub     sp, sp, #32
    mov     x20, sp        // buffer
    mov     x21, x0        // value toprint

    // Handle zero
    cmp     x21, #0
    b.ne    check_sign
    ldr     x0, =zerochar
    mov     x1, x0
    mov     x2, #1
    mov     x8, #64
    mov     x0, #1
    svc     0
    b       print_newline

check_sign:
    cmp     x21, #0
    b.ge    convert
    ldr     x0, =minuschar
    mov     x1, x0
    mov     x2, #1
    mov     x8, #64
    mov     x0, #1
    svc     0
    sub     x21, xzr, x21   // Manual negate: x21 = 0 - x21

convert:
    mov     x22, x21       // working copy
    mov     x23, #0        // digit count

convert_loop:
    mov     x24, #10
    udiv    x25, x22, x24  // x25 = x22 / 10
    mul     x26, x25, x24  // x26 = x25 * 10
    sub     x26, x22, x26  // x26 = x22 - x26 (remainder)
    add     x26, x26, 48
    strb    w26, [x20, x23]
    add     x23, x23, #1
    mov     x22, x25
    cbnz    x22, convert_loop

print_loop:
    subs    x23, x23, #1
    blt     print_newline
    ldrb    w27, [x20, x23]
    strb    w27, [x20, #0] // copy digit to buffer start
    mov     x0, #1         // stdout
    mov     x1, x20        // buffer address
    mov     x2, #1         // one byte
    mov     x8, #64        // write syscall
    svc     0
    b       print_loop

print_newline:
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
