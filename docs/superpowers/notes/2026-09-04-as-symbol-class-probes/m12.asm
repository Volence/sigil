	cpu	68000
	padding	off
	org	0
; --- a CONSTANT declared inside a `rept`: does each iteration count? ---
; m6 asked this of a `set` (legal, it is the reassignable class). The constant
; form is the open cell: the body runs twice, so the second run is a second
; declaration of the same name unless `rept` is special.
	rept	2
Ar	equ	7
	endm
; --- the same, with a colon LABEL, whose two values genuinely differ ---
	rept	2
Br:
	dc.w	$1111
	endm
; --- and once, as the control: one iteration must be silent ---
	rept	1
Cr	equ	7
	endm
	dc.w	$4444
