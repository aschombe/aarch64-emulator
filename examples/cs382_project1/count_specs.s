.global count_specs

count_specs: //X0 = char array
MOV X15, X0     //X15 = char array 
MOV X0, 0       //X0 = Count of %a
MOV X16, 37     //X16 = ascii for %
MOV X17, 97     //X17 = ascii for a
MOV X18, 0     //X18 = array offset

loop:               //X1 = spot for loading
LDRB W1, [X15, X18] //Gets the next char
CMP X1, XZR         //First compares char to 0
B.EQ endLoop        //End loop if hit the end
CMP X1, X16         //Next compares char to %
B.EQ checkforA      //If equal, then check for A
ADD X18, X18, 1     //If not, increase the offset
B loop              //And Loop again

checkforA:
ADD X18, X18, 1     //Increases offset by 1
LDRB W1, [X15, X18] //Loads following byte
CMP X1, XZR         //First compares char to 0
B.EQ endLoop        //And ends the loop if they are
CMP X1, X17         //Then compares to a
B.EQ addCount       //Adds 1 to the count if they are equal
B loop              //Loops again without increasing offset cause it was already increased

addCount:
ADD X0, X0, 1       //Adds 1 to the count
B loop              //And loops again without increasing offset cause it was already increased

endLoop:
RET                 //Just returns, as the count is already in X0



/*
    Declare .data here if you need.
*/
