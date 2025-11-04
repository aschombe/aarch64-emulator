
/*
 * Do not change (add/remove/modify) anything in this file
 * Except the __values__ of the data for your own tests 
 */


.global src_str
.global dst_str

.data
 src_str: .string "I love assembly! <3" // source string
.bss
 dst_str: .skip 100 // destination string
