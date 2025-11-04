.text
.global _start

_start:

ADR X11, src_str //Address of source stored into X11
ADR X12, dst_str //Address of destination stored into X12
MOV X13, 0       //Counter for the offset

Loop:
LDRB W14, [X11, X13] //Loads the next byte of sorce into W14
STRB W14, [X12, X13] //Stores that into the correct spot of destination
ADD X13, X13, 1   //Increases X13 by 1
CBZ W14, Exit       //Checks if hit end of string
B Loop              //Loops

Exit:
MOV X0, 1       /* status <- 1 */
MOV X1, X12     /* Loads address of destination into register X1 */
MOV X2, X13     /* Moves the length into register X2 */
MOV X8, 64      /* Print system call #64 */
SVC  0          //System call yay 

MOV  X0, 0    
MOV  X8, 93     
SVC  0
