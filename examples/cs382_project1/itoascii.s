.global itoascii

itoascii:   //X0 = number

MOV X10, 10 //X10 = 10 for multiplying
MOV X11, 1 //X11 = 1, 10, 100, etc for dividing
MOV X1, 0   //X1 = Length of number
MOV X3, 0  //X3 = Number that has been extracted
MOV X12, -1 //X12 = -1 for compare
ADR X13, buffer //X13 = buffer address
STR XZR, [X13]   //Resets it I hope
MOV X14, 0        //Buffer offset

CBZ X0, zero      //Checks if the number is 0 (special case)

findLength:
UDIV X2, X0, X11       //X2 = temp spot, divides the number by next power of 10
CBZ X2, mainLoop //If nothing, then end loop
ADD X1, X1, 1            //Adds 1 to the length
MUL X11, X11, X10    //Multiplies X11 by the next power of 10
B findLength

zero:
ADD X1, X1, 1  //If the number started as 0, add 1 to the length
B mainLoop     //And skip to the main loop

mainLoop:
SUB X1, X1, 1  //Subtracts 1 from length, as the extra is not needed
CMP X1, X12    //Compares current length to -1
B.EQ EndLoop   //If it is -1, then we are done here
MOV X4, X1     //Puts length into X4 for counting divisions
MOV X5, X0     //Puts the number into X5 for dividing
SUB X5, X5, X3 //Subtracts the part of the number that has already been extracted

divideLoop:
CBZ X4, endDivideLoop //If divide loop is done, then exit
UDIV X5, X5, X10     //Divides number by 10
SUB X4, X4, 1         //Subtracts 1 from the number of divisions needed
B divideLoop

endDivideLoop:
ADD X6, X5, 48    //Turns current digit to askii and stores it in X6
STRB W6, [X13, X14] //Stores the ackii bit into the next spot of buffer
ADD X14, X14, 1      //Adds 1 to the offset
MOV X4, X1           //Gets a temp length again

multLoop:
CBZ X4, endMultLoop //If mult loop is done, then exit
MUL X5, X5, X10         //Multiplies the number by 10
SUB X4, X4, 1         //Subtracts 1 from the number of mults needed
B multLoop

endMultLoop:
ADD X3, X3, X5    //Adds the extracted number to X3
B mainLoop

EndLoop:
MOV X0, X13    //Puts the address in X0 for return
RET




.data
    /* Put the converted string into buffer,
       and return the address of buffer */
buffer: .fill 128, 1, 0


