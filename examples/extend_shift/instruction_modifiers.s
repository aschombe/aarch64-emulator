.text
.global _start
_start:
    // Values for testing
    mov x1, #0xFFFFFFFFFFFFFFFF    // All bits high
    mov x2, #0x8000000000000000    // Only high bit
    mov w3, #0xFFFF                // 16 bits, all 1
    mov x4, #0x1                   // Lowest bit
    mov x5, #0xA5A5A5A5A5A5A5A5    // Pattern

    // Arithmetic with register shifts
    add x10, x1, x4, lsl #63      // Test max shift (should shift to MSB)
    add x11, x1, x4, lsl #0       // Shift by 0 (should not change)

    sub x12, x2, x4, lsl #63      // High bit - value in MSB
    sub x13, x4, x2, lsr #63      // Lowest bit - high bit shifted down

    // Extended register
    add x14, x4, w3, uxtw #1      // w3=0xFFFF, <<1 = 0x1FFFE
    sub x15, x4, w3, sxtw #2      // w3=0xFFFF (sign-extend=65535), <<2 = 0x3FFFC

    // Logical with shift/extend
    and x16, x1, x5, lsr #32      // Mask pattern, test logical shift
    orr x17, x2, w3, uxtb #7      // w3=0xFF after uxtb, <<7=0x7F80, OR with x2
    eor x18, x5, w3, sxth #12     // w3=0xFFFF (sign-extend=65535), <<12
    bic x19, x1, w3, uxtw #0      // Zero-extend, shift by 0: should mask out lower 16 bits

    // Immediate shift (should only shift up to legal width)
    add x20, x1, #1, lsl #63      // Shift immediate by max
    add x21, x1, #1, lsl #0       // Shift immediate by 0

    // NEGS, ADDS, SUBS with shifts
    neg x22, x4, lsl #5
    negs x23, x4, lsr #3
    adds x24, x1, x4, lsl #1
    subs x25, x2, x4, lsr #63

    // Loads & stores: extended/shifted
    mov x6, #0x100
    mov x7, #0x200
    mov w8, #0x7F
    str x4, [x6, x7, lsl #1]      // Store at address x6 + (x7 << 1)
    ldr x27, [x6, x7, lsl #1]     // Load back to x27
    str x4, [x6, w8, uxtw #2]     // Store at address x6 + (w8 << 2)
    ldr x28, [x6, w8, uxtw #2]    // Load back to x28

    mov x0, #77   // exit code
    mov x8, #93   // Exit syscall
    svc #0
