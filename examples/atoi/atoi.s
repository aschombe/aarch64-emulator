.text
.global _start

/* Add your code here */
_start:
    ADR X0, numstr     // Load the address of the string
    ADR X1, number      // Load the address of the integer result
    MOV X2, 0           //Current Bits
    MOV X3, 0           //Integer Result
    MOV X10, 10        //Assembly Dumb

parse_loop:
    LDRB W4, [X0,X2]       // Load the current character
    CBZ X4, almost_exit   // Check for the null terminator (end of the string)
    
    MUL X3, X3, X10       //Multiply by 10
    SUB X5, X4, 48        // Convert ASCII character to integer and 48 is not being subtracted
    ADD X3, X3, X5       // Add the digit to the integer
    ADD X2, X2, 1        // Move to the next character in the string
    B parse_loop

almost_exit:
    STR X3, [X1]

/* Do not change any part of the following code */
exit:
    // MOV  X0, 1
    // ADR  X1, number
    // MOV  X2, 8
    // MOV  X8, 64
    // SVC  0
    MOV  X0, 0
    MOV  X8, 93
    SVC  0
    /* End of the code. */
