	cpu	68000
	padding	off
	org	0
; --- a MACRO invoked twice, each expansion declaring the same name ---
; m6 measured a macro declaring over a name the CALLER made a constant. Here
; both declarations come from the expansion, which is the shape a real source
; hits when a helper macro is used more than once.
mequ	macro
Am	equ	7
	endm
	mequ
	mequ
; --- the same, with a colon label placed at two DIFFERENT addresses ---
mlbl	macro
Bm:
	dc.w	$1111
	endm
	mlbl
	mlbl
; --- and the documented escape: `local` inside the macro ---
mloc	macro
	local	Cm
Cm:
	dc.w	$2222
	endm
	mloc
	mloc
	dc.w	$4444
