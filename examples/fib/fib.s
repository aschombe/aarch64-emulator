	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 26, 0	sdk_version 26, 1
	.globl	_fibbonacci                     ; -- Begin function fibbonacci
	.p2align	2
_fibbonacci:                            ; @fibbonacci
	.cfi_startproc
; %bb.0:
	sub	sp, sp, #32
	stp	x29, x30, [sp, #16]             ; 16-byte Folded Spill
	add	x29, sp, #16
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	str	w0, [sp, #8]
	ldr	w8, [sp, #8]
	cbnz	w8, LBB0_2
	b	LBB0_1
LBB0_1:
	stur	wzr, [x29, #-4]
	b	LBB0_5
LBB0_2:
	ldr	w8, [sp, #8]
	subs	w8, w8, #1
	b.ne	LBB0_4
	b	LBB0_3
LBB0_3:
	mov	w8, #1                          ; =0x1
	stur	w8, [x29, #-4]
	b	LBB0_5
LBB0_4:
	ldr	w8, [sp, #8]
	subs	w0, w8, #1
	bl	_fibbonacci
	str	w0, [sp, #4]                    ; 4-byte Folded Spill
	ldr	w8, [sp, #8]
	subs	w0, w8, #2
	bl	_fibbonacci
	mov	x8, x0
	ldr	w0, [sp, #4]                    ; 4-byte Folded Reload
	add	w8, w0, w8
	stur	w8, [x29, #-4]
	b	LBB0_5
LBB0_5:
	ldur	w0, [x29, #-4]
	ldp	x29, x30, [sp, #16]             ; 16-byte Folded Reload
	add	sp, sp, #32
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:
	sub	sp, sp, #96
	stp	x29, x30, [sp, #80]             ; 16-byte Folded Spill
	add	x29, sp, #80
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	adrp	x8, ___stack_chk_guard@GOTPAGE
	ldr	x8, [x8, ___stack_chk_guard@GOTPAGEOFF]
	ldr	x8, [x8]
	stur	x8, [x29, #-8]
	str	wzr, [sp, #28]
	str	wzr, [sp, #24]
	b	LBB1_1
LBB1_1:                                 ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB1_5 Depth 2
                                        ;     Child Loop BB1_8 Depth 2
	ldr	w8, [sp, #24]
	subs	w8, w8, #10
	b.ge	LBB1_14
	b	LBB1_2
LBB1_2:                                 ;   in Loop: Header=BB1_1 Depth=1
	ldr	w0, [sp, #24]
	bl	_fibbonacci
	str	w0, [sp, #20]
	str	wzr, [sp, #16]
	ldr	w8, [sp, #20]
	cbnz	w8, LBB1_4
	b	LBB1_3
LBB1_3:                                 ;   in Loop: Header=BB1_1 Depth=1
	ldrsw	x9, [sp, #16]
	mov	x8, x9
	add	w8, w8, #1
	str	w8, [sp, #16]
	sub	x8, x29, #28
	add	x9, x8, x9
	mov	w8, #48                         ; =0x30
	strb	w8, [x9]
	b	LBB1_12
LBB1_4:                                 ;   in Loop: Header=BB1_1 Depth=1
	ldr	w8, [sp, #20]
	str	w8, [sp, #12]
	str	wzr, [sp, #8]
	b	LBB1_5
LBB1_5:                                 ;   Parent Loop BB1_1 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ldr	w8, [sp, #12]
	subs	w8, w8, #0
	b.le	LBB1_7
	b	LBB1_6
LBB1_6:                                 ;   in Loop: Header=BB1_5 Depth=2
	ldr	w8, [sp, #12]
	mov	w9, #10                         ; =0xa
	sdiv	w10, w8, w9
	mul	w10, w10, w9
	subs	w8, w8, w10
	add	w8, w8, #48
	ldrsw	x11, [sp, #8]
	mov	x10, x11
	add	w10, w10, #1
	str	w10, [sp, #8]
	add	x10, sp, #32
	strb	w8, [x10, x11]
	ldr	w8, [sp, #12]
	sdiv	w8, w8, w9
	str	w8, [sp, #12]
	b	LBB1_5
LBB1_7:                                 ;   in Loop: Header=BB1_1 Depth=1
	ldr	w8, [sp, #8]
	subs	w8, w8, #1
	str	w8, [sp, #4]
	b	LBB1_8
LBB1_8:                                 ;   Parent Loop BB1_1 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ldr	w8, [sp, #4]
	tbnz	w8, #31, LBB1_11
	b	LBB1_9
LBB1_9:                                 ;   in Loop: Header=BB1_8 Depth=2
	ldrsw	x9, [sp, #4]
	add	x8, sp, #32
	add	x8, x8, x9
	ldrb	w8, [x8]
	ldrsw	x10, [sp, #16]
	mov	x9, x10
	add	w9, w9, #1
	str	w9, [sp, #16]
	sub	x9, x29, #28
	add	x9, x9, x10
	strb	w8, [x9]
	b	LBB1_10
LBB1_10:                                ;   in Loop: Header=BB1_8 Depth=2
	ldr	w8, [sp, #4]
	subs	w8, w8, #1
	str	w8, [sp, #4]
	b	LBB1_8
LBB1_11:                                ;   in Loop: Header=BB1_1 Depth=1
	b	LBB1_12
LBB1_12:                                ;   in Loop: Header=BB1_1 Depth=1
	ldrsw	x9, [sp, #16]
	mov	x8, x9
	mov	w0, #1                          ; =0x1
	add	w8, w8, #1
	str	w8, [sp, #16]
	sub	x1, x29, #28
	mov	x8, x1
	add	x9, x8, x9
	mov	w8, #10                         ; =0xa
	strb	w8, [x9]
	ldrsw	x2, [sp, #16]
	bl	_write
	b	LBB1_13
LBB1_13:                                ;   in Loop: Header=BB1_1 Depth=1
	ldr	w8, [sp, #24]
	add	w8, w8, #1
	str	w8, [sp, #24]
	b	LBB1_1
LBB1_14:
	ldr	w8, [sp, #28]
	str	w8, [sp]                        ; 4-byte Folded Spill
	ldur	x9, [x29, #-8]
	adrp	x8, ___stack_chk_guard@GOTPAGE
	ldr	x8, [x8, ___stack_chk_guard@GOTPAGEOFF]
	ldr	x8, [x8]
	subs	x8, x8, x9
	b.eq	LBB1_16
	b	LBB1_15
LBB1_15:
	bl	___stack_chk_fail
LBB1_16:
	ldr	w0, [sp]                        ; 4-byte Folded Reload
	ldp	x29, x30, [sp, #80]             ; 16-byte Folded Reload
	add	sp, sp, #96
	ret
	.cfi_endproc
                                        ; -- End function
.subsections_via_symbols
