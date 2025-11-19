.text
.global _start

_start:
    mov  x0, #0x7ffffff0      // MOV immediate to x0
    movk x1, #0x1234          // MOVK immediate to x1
    movz x2, #0xabcd          // MOVZ immediate to x2
    movn x3, #0x5555          // MOVN immediate to x3

    add  x4, x0, x1           // ADD registers
    add  x5, x2, #100         // ADD with immediate

    sub  x6, x4, x5           // SUB registers
    subs x7, x4, #45          // SUBS with immediate

    neg  x8, x7               // NEG
    negs x9, x8               // NEGS

    nop                       // NOP

    str  x4, [sp, #-16]!      // Store with pre-indexed addressing
    ldr  x10, [sp], #16       // Load with post-indexed addressing
    ldr  w11, [x2, #8]        // Load 32-bit from address x2 + 8

    add  x12, xzr, x5         // Use xzr (always zero), result is just x5

    mov  x13, #77             // MOV 77 into x13
    mov  x14, #99             // MOV 99 into x14

    // Set exit code to the calculation result (for example x12)
    mov  x0, x12              // Use the result of earlier arithmetic as exit code
    mov  x8, #93              // syscall: exit
    svc  #0                   // exit(x0)

