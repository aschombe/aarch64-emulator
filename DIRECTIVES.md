# Supported Directives

## Sections
- .text
- .data
- .bss
- .rodata

## Data Directives
- General Form:
    ```
    label: <directive> <values>
    ```
    ```
- .byte:
    - ```.byte <value1>, <value2>, ...```
    - Supports: decimal, hexadecimal (0x), octal (0o), and character literals.
- .single/.float:
    - ```.single <value1>, <value2>, ...```
    - ```.float <value1>, <value2>, ...```
- .double/.doubleword:
    - ```.double <value1>, <value2>, ...```
    - ```.doubleword <value1>, <value2>, ...```
- .quad/.dword:
    - ```.quad <value1>, <value2>, ...```
    - ```.dword <value1>, <value2>, ...```
    - 64-bit
- .word/.int:
    - ```.word <value1>, <value2>, ...```
    - ```.int <value1>, <value2>, ...```
    - 32-bit
- .string/.asciz/.asciiz:
    - ```.string "your string here"```
    - ```.asciz "your string here"```
    - ```.asciiz "your string here"```
    - Null-terminated string
- .ascii:
    - ```.ascii "your string here"```
    - Non-null-terminated string
- .skip/.space:
    - ```.skip <number_of_bytes>```
    - ```.space <number_of_bytes>```
    - Allocates specified number of bytes without initializing them
- .fill:
    - ```.fill <count>, <size>, <value>```
    - Fills memory with a specified value repeated a certain number of times
- .balign:
    - ```.balign <alignment>```
    - Aligns the next data to the specified byte boundary
- .rept/.endr:
    - ```.rept <count>```
    - ```.endr```
    - Repeats the enclosed directives a specified number of times

## Other
- .globl/.global:
    - ```.globl <symbol>```
    - ```.global <symbol>```
    - Declares a global symbol
- .extern:
    - ```.extern <symbol>```
    - Declares an external symbol
- .equ:
    - ```.equ <symbol>, <value>```
    - Defines a constant value for a symbol
