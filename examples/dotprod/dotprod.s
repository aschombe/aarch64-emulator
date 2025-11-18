.global _start
.text

_start:

ADR X0, vec1
ADR X1, vec2
ADR X2, dot

LDR X3, [X0, 0]
LDR X4, [X1, 0]
MUL X5, X3, X4

LDR X3, [X0, 8]
LDR X4, [X1, 8]
MUL X6, X3, X4
ADD X5, X5, X6

LDR X3, [X0, 16]
LDR X4, [X1, 16]
MUL X6, X3, X4
ADD X5, X5, X6
MOV X0, X5
// STR X5, [X2, 0]

// MOV X8, 64
// MOV X0, 1
// ADR X1, dot
// MOV X2, 8
// SVC 0

MOV X8, 93
SVC 0

.data
vec1: .quad 10, 20, 30
vec2: .quad 1, 2, 3
dot: .quad 0
