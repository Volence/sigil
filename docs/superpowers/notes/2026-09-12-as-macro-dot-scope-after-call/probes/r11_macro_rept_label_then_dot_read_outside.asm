; body `rept 1` / `Inner:` / `endr`, then `.y:`; read `Inner.y` after the call
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
	endm
	mac
	dc.w	Inner.y	; REF
	dc.w	$4444
