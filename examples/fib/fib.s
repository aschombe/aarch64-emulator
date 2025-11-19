.data
buf:    .space 32

.text
.global _start

_start:
    mov     x0, #0          // fib_n_minus_2 = 0
    mov     x1, #1          // fib_n_minus_1 = 1
    mov     x2, #0          // i = 0
    mov     x3, #100        // limit = 100

loop:
    // Print current value (x0)
    mov     x4, x0          // Value to print
    ldr     x5, =buf
    bl      print_uint64

    // Print newline
    mov     x8, #64         // sys_write
    mov     x0, #1          // fd=1 (stdout)
    ldr     x1, =newline
    mov     x2, #1
    svc     #0

    // Next Fibonacci
    mov     x6, x0          // temp = fib_n_minus_2
    add     x0, x1, x0      // x0 = fib_n_minus_1 + fib_n_minus_2
    mov     x1, x6          // fib_n_minus_1 = old fib_n_minus_2

    add     x2, x2, #1      // i++
    cmp     x2, x3
    blt     loop

    // Exit
    mov     x8, #93         // sys_exit
    mov     x0, #0
    svc     #0

// Print unsigned int in x4, buffer at x5
// Clobbers x6-x9
print_uint64:
    mov     x6, x5
    mov     x7, #0
    mov     x8, x4
print_digit:
    mov     x9, #10
    udiv    x4, x8, x9
    msub    x9, x4, x9, x8
    add     x9, x9, #'0'
    strb    w9, [x6], #1
    mov     x8, x4
    add     x7, x7, #1
    cmp     x4, #0
    bne     print_digit

    sub     x6, x6, #1
rev_loop:
    ldrb    w9, [x5], #1
    strb    w9, [x6], #0
    sub     x7, x7, #1
    cbnz    x7, rev_loop

    ldr     x1, =buf
    mov     x2, x6
    sub     x2, x2, x1
    mov     x2, x6
    ldr     x1, =buf
    sub     x2, x2, x1
    mov     x1, x6
    ldr     x6, =buf
    sub     x1, x1, x6
    mov     x2, x1
    ldr     x1, =buf

    // Actually use this:
    ldr     x1, =buf
    sub     x2, x6, x5
    mov     x8, #64
    mov     x0, #1
    svc     #0
    ret

.rodata
newline: .asciz "\n"

