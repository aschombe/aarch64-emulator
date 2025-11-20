
.global _start

_start:
    mov x0, #1
    b skip         // branch to label 'skip'
    mov x0, #2     // should be skipped
skip:
    add x0, x0, #10

    cmp x0, #11
    b.eq done      // branch if equal to label 'done'
    mov x0, #3     // should be skipped

done:
    mov x8, #93
    svc #0
