; body `Lp:` then `.x:`, read `Lp.x` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp:	dc.w	$2222
.x:	dc.w	$3333
	endm
	mac
	dc.w	Lp.x	; REF
	dc.w	$4444
