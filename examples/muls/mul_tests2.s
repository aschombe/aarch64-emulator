.text
.global _start
_start:
    mov x1, #3
    mov x2, #5
    mov x3, #2
    madd x4, x1, x2, x3    // x4 = 3*5 + 2 = 17
    msub x5, x1, x2, x3    // x5 = 3*5 - 2 = 13
    mneg x6, x1, x2        // x6 = -(3*5) = -15 (0xFFFFFFFFFFFFFFF1)

    mov w7, #-10
    mov w8, #-5
    mov x9, #100
    smaddl x10, w7, w8, x9   // x10 = (-10)*(-5)+100 = 150
    smsubl x11, w7, w8, x9   // x11 = (-10)*(-5)-100 = -50
    smnegl x12, w7, w8       // x12 = -((-10)*(-5)) = -50

    mov w13, #7
    mov w14, #9
    mov x15, #3
    umaddl x16, w13, w14, x15  // x16 = 7*9+3 = 66
    umsubl x17, w13, w14, x15  // x17 = 7*9-3 = 60
    umnegl x18, w13, w14       // x18 = -(7*9) = -63

    mov w19, #-9
    smull x20, w19, w8         // x20 = (-9)*(-5) = 45
    smulh x21, x1, x2          // x21 = high bits of 3*5 (should be 0)
    umull x22, w13, w14        // x22 = 7*9 = 63
    umulh x23, x14, x13        // x23 = high bits of 9*7

    mov x8, #93
    mov x0, #42
    svc #0

