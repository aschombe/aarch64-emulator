.text
.global _start
.extern pringle

_start:
    adr x0, input_str   // Load address of input string
    adr x1, array1      // Load address of array1
    adr x2, len1        // Load length of array1
    ldr x2, [x2]        // Dereference length
    bl pringle          // Call pringle function

    // exit
    mov x0, 0
    mov x8, 93
    svc 0


.data
input_str: .string "Array 1: %a\n"
array1: .quad 10, 20, 30, 40, 50
len1: .quad 5
