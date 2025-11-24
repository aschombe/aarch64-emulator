.data
// --- Case 1: .org 0 and zero base test
.org 0
zero_start:
    .byte 7, 8, 9

// --- Case 2: Unaligned org with mixed data
.org 0x1001
unaligned:
    .byte 0x11
    .word 0xABCD

// --- Case 3: org with gaps and check untouched memory
.org 0x2000
first:
    .byte 0x22

.org 0x2100
second:
    .byte 0x33

// --- Case 4: Overlapping .org (should overwrite)
.org 0x3000
overlap1:
    .word 0x12345678

.org 0x3000
overlap2:
    .word 0xA5A5A5A5

// --- Case 5: Arrays & multi-word
.org 0x4000
arr1:
    .word 0x1111
    .word 0x2222

.org 0x4010
arr2:
    .quad 0x3333333344444444
    .quad 0x5555555566666666

.text
.global _start
_start:
    // --- Check zero base
    ldrb x10, [0]          // 7
    ldrb x11, [1]          // 8
    ldrb x12, [2]          // 9
    add  x10, x10, x11     // 7+8=15
    add  x10, x10, x12     // 15+9=24

    // --- Check unaligned write
    ldrb x13, [0x1001]     // 0x11
    ldr  w14, [0x1002]     // 0xABCD (43725)
    add  x10, x10, x13     // 24+0x11=41
    add  x10, x10, x14     // 41+43725=43766

    // --- Gap case, untouched memory
    ldrb x15, [0x2000]     // 0x22
    ldrb x16, [0x2001]     // gap (should be 0)
    ldrb x17, [0x2100]     // 0x33
    add  x10, x10, x15     // +0x22
    add  x10, x10, x16     // +0 (should not change)
    add  x10, x10, x17     // +0x33

    // --- Overlap: should be new value
    ldr  w18, [0x3000]     // 0xA5A5A5A5
    add  x10, x10, x18

    // --- Array layout and separate base/align
    ldr  w19, [0x4000]     // 0x1111
    ldr  w20, [0x4004]     // 0x2222
    ldr  x21, [0x4010]     // 0x3333333344444444
    ldr  x22, [0x4018]     // 0x5555555566666666
    add  x10, x10, x19
    add  x10, x10, x20
    add  x10, x10, x21
    add  x10, x10, x22

    // Exit with final result (should be unique for expected values)
    mov  x0, x10
    mov  x8, #93
    svc 0
