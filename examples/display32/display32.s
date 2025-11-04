.text
.global _start

_start:
    adr x0, num
    ldr w0, [x0]
    bl display_32

    mov w0, 10
    bl display

    mov x0, 0
    mov x8, 93
    svc 0

display_32:
    sub sp, sp, 24
    str x30, [sp]
    str x19, [sp, 8]
    str x20, [sp, 16]
    
    mov w19, w0
    mov w20, 31

    loop:
        mov w0, w19
        lsr w0, w0, w20
        and w0, w0, 1
        
        bl display
        
        sub w20, w20, 1
        
        cmp w20, -1
        b.gt loop

    ldr x30, [sp]
    ldr x19, [sp, 8]
    ldr x20, [sp, 16]
    add sp, sp, 24
    ret

display:
    cmp w0, 10
    beq skip_convert
    add w0, w0, 48

    skip_convert:
    sub sp, sp, 8
    str w0, [sp]

    mov x0, 1
    mov x1, sp
    mov x2, 1
    mov x8, 64
    svc 0
    add sp, sp, 8

    ret

.data
num: .word 382
