; body `rept 1` / `Inner:` / `endr`, then `.y:`; the body reads `.y`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
	rept	1
Inner:	dc.w	$2222
	endr
.y:	dc.w	$2223
	dc.w	.y	; REF
	endm
	mac
	dc.w	$4444
