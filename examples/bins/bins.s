.text
.global _start

_start:

ADR X11, arr    //Address of array in X11

ADR X12, length //Address of length in X12
LDR X12, [X12]  //Loads value of length into X12

ADR X13, target //Address of target into X13
LDR X13, [X13]  //Loads value of target into X13

ADR X14, msg1   //Loads address of success msg into X14
ADR X15, msg1len //Address of msg1length in X15
LDR X15, [X15]  //Loads value of msg1length into X15

ADR X16, msg2   //Loads address of fail msg into X16
ADR X17, msg2len //Address of msg2length in X17
LDR X17, [X17]  //Loads value of msg2length into X17

MOV X28, 0      //Moves 0 to X28 for the start index of the array
MOV X30, 1      //Moves 1 to X30
SUB X30, X12, X30 //Subtracts 1 from length and puts it in X30 for the last index
MOV X18, 8      //Puts 8 in X18 for multiplying later
MOV X21, 1      //Puts 1 in X21 to use later

Loop:
ADD X29, X30, X28 //Finds the sum of the right and left bound
LSR X29, X29, X21 //Shifts right by 1 to divide by 2
MUL X27, X29, X18 //Finds the offset for the index and puts it in X27 (mid x 8)
LDR X27, [X11, X27] //Puts the middle number into X27 (x11 is array)
CMP X27, X13        //Checks if found the target
B.EQ printSucc      //If found, print success
CMP X28, X30        //Compares the upper and lower bound
B.EQ printFail      //If they are equal, print fail
CMP X27, X13        //Else compare the target again
B.GT shrinkUpperBound//If number is bigger than target, upper bound needs to be brought down
B raiseLowerBound   //Else, raise the lower bound

shrinkUpperBound:
SUB X29, X29, X21 //Subtracts 1 from the middle index
MOV X30, X29    //Sets the upper bound to new upper bound
CMP X30, X28    //Compares upper bound to lower bound
B.LT printFail  //If upper < lower, fail
B Loop          //More!

raiseLowerBound:
ADD X29, X29, X21 //Adds 1 from the middle index
MOV X28, X29    //Sets the lower bound to new lower bound
CMP X28, X30    //Compares lower bound to upper bound
B.GT printFail  //If lower > upper, fail
B Loop          //Again!

printSucc:
MOV X0, 1       /* status <- 1 */
MOV X1, X14     /* Loads address of msg1 into register X1 */
MOV X2, X15     /* Moves the length into register X2 */
MOV X8, 64      /* Print system call #64 */
SVC  0          //System call yay 
B Exit          //End

printFail:
MOV X0, 1       /* status <- 1 */
MOV X1, X16     /* Loads address of msg2 into register X1 */
MOV X2, X17     /* Moves the length into register X2 */
MOV X8, 64      /* Print system call #64 */
SVC  0          //System call yay 

Exit:
MOV  X0, 0    
MOV  X8, 93     
SVC  0

.data
    msg1len:  .quad 24
    msg2len:  .quad 28
