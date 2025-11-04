
.global concat_array

concat_array: //X0 = array address, X1 = length of array
SUB SP, SP, 88
STR X30, [SP]
STR X20, [SP, 8]
STR X21, [SP, 16]
STR X22, [SP, 24]
STR X23, [SP, 32]
STR X24, [SP, 40]
STR X25, [SP, 48]
STR X26, [SP, 56]
STR X27, [SP, 64]
STR X28, [SP, 72]
STR X19, [SP, 80]

MOV X20, X0  //X20 = addres of array for safe-ish keeping
MOV X21, 0   //X21 = offset for array
ADR X22, concat_array_outstr //X22 = address of output
MOV X23, 32  //X23 = ascii space
MOV X24, X1  //X24 = array length for safe-ish keeping
MOV X25, 0   //X25 = Counter for array length
MOV X26, 0   //X26 = offset of output
MOV X27, 0   //X27 = length of ascii that is returned
MOV X28, 0   //X28 = temp spot for ascii chars

Loop:
CMP X24, X25 //Compares length with current counter
B.EQ exitLoop //If equal, then exit loop
MOV X0, 0         //Resets X0 I hope this works idk
LDR X0, [X20, X21] //X0 = next array num
BL itoascii        //Sends the number and turns it into ascii
MOV X19, X0      //X19 = Ascii address
LDR X0, [X0]       //Loads the value of the ascii instead of the address
STR X0, [X22, X26] //Stores the ascii into the output string

countString:
LDRB W28, [X19, X27] //Stores the next ascii into W28
CMP X28, 0x00        //Checks if at null
CBZ X28, endCount  //If hit end, then exit loop
ADD X27, X27, 1    //Else add 1 to the count and offset
B countString      //And go again

endCount:
ADD X26, X26, X27   //Adds string length to the output offset
MOV X27, 0           //Reset the counter
STR X23, [X22, X26] //Stores a space into the output
ADD X25, X25, 1   //Adds 1 to counter
ADD X21, X21, 8   //Adds 8 to the offset of array
ADD X26, X26, 1   //Adds 1 to offset of output
B Loop

exitLoop:
MOV X0, X22 //Loads the address of the output into X0 for return

LDR X30, [SP]
LDR X20, [SP, 8]
LDR X21, [SP, 16]
LDR X22, [SP, 24]
LDR X23, [SP, 32]
LDR X24, [SP, 40]
LDR X25, [SP, 48]
LDR X26, [SP, 56]
LDR X27, [SP, 64]
LDR X28, [SP, 72]
LDR X19, [SP, 80]
ADD SP, SP, 88
RET

.data
    /* Put the converted string into concat_array_outstrer,
       and return the address of concat_array_outstr */
concat_array_outstr: .fill 1024, 1, 0

