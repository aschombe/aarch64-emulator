.global pringle

pringle: //X0 = string, X1=array1, X2=length1, ...
SUB SP, SP, 152
STR X30, [SP]
STR X19, [SP, 8]
STR X20, [SP, 16]
STR X21, [SP, 24]
STR X22, [SP, 32]
STR X23, [SP, 40]
STR X24, [SP, 48]
STR X25, [SP, 56]
STR X26, [SP, 64]
STR X27, [SP, 72]
STR X28, [SP, 80]

STR X0, [SP, 88]            // <-- EXTRA CREDIT STUFF
STR X1, [SP, 96]            //Stores all of the register
STR X2, [SP, 104]           //parameters on the stack just
STR X3, [SP, 112]           //before any extras that would
STR X4, [SP, 120]           //be there from before the proc call
STR X5, [SP, 128]           //so that I can just keep pulling
STR X6, [SP, 136]           //arrays and lengths from the stack
STR X7, [SP, 144]           //for as many %a's there are.

ADR X19, outstr //X19 = Address of outstr
STR XZR, [X19]  //Clears outstr
MOV X20, X0     //X20 = input string
MOV X21, 0      //X21 = input string offset
MOV X23, 0      //X23 = number of chars printed
MOV X24, 0      //X24 = Outstr offset
MOV X25, 37     //X25 = ascii for %
MOV X26, 97     //X26 = ascii for a
MOV X27, 96      //X27 = stack offset 

loop:
LDRB W22, [X20, X21] //X22 = temp place for input char
CMP X22, XZR        //Checks if char is 0
B.EQ endLoop        //If it is, end loop
CMP X22, X25         //Next compares char to %
B.EQ checkforA      //If equal, then check for A
STRB W22, [X19, X24] //If not, store char into outstr
ADD X23, X23, 1     //Add 1 to chars
ADD X21, X21, 1     //Increase the offset
ADD X24, X24, 1     //For Both
B loop              //And Loop again

checkforA:
ADD X21, X21, 1     //Increases offset by 1
LDRB W22, [X20, X21] //Loads following byte
CMP X22, X26         //Compares to a
B.EQ spotFound       //If yes, then a spot has been found
SUB X21, X21, 1      //Otherwise need to reverse
LDRB W22, [X20, X21]  //Load the previous byte again
STRB W22, [X19, X24] //And store it into the output
ADD X23, X23, 1     //Add 1 to chars
ADD X21, X21, 1     //Increase the offset
ADD X24, X24, 1     //For Both
B loop              //And Loop again

spotFound:
ADD X21, X21, 1    //Adds 1 to the offset early
LDR X0, [SP, X27]  //Gets the next array from the stack     <-- Extra credit
ADD X27, X27, 8    //Adds 8 to the stack offset             <-- Extra credit
LDR X1, [SP, X27]  //Gets the next length from the stack    <-- Extra credit
ADD X27, X27, 8    //Adds 8 to the stack offset again!      <-- Extra credit
BL concat_array    //Turns it into ascii
MOV X8, 0          //X8 = offset for concat to output
MOV X9, 0          //X9 = temp spot for char

concatToOutputLoop:
LDRB W9, [X0, X8]   //Loads next char
CBZ X9, loop        //If hits the end, can loop again
STRB W9, [X19, X24] //Else, store that char into the output
ADD X23, X23, 1     //Add 1 to chars
ADD X8, X8, 1        //Increase the offset
ADD X24, X24, 1     //For Both
B concatToOutputLoop //And Loop again

endLoop:
STRB W22, [X19, X24] //Adds the final 0 to output

MOV X0, 1      
MOV X1, X19 //X1 = ADR of outstr
MOV X2, X23 //X2 = length  
MOV X8, 64      
SVC 0

MOV X0, X23         //puts number of chars printed into X0 for return

LDR X30, [SP]
LDR X19, [SP, 8]
LDR X20, [SP, 16]
LDR X21, [SP, 24]
LDR X22, [SP, 32]
LDR X23, [SP, 40]
LDR X24, [SP, 48]
LDR X25, [SP, 56]
LDR X26, [SP, 64]
LDR X27, [SP, 72]
LDR X28, [SP, 80]
ADD SP, SP, 152
RET

/*
    Declare .data here if you need.
*/
.data
outstr: .fill 1024, 1, 0
