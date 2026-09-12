; body `Lp:` `.x:` then `dc.w Lp.x` inside
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp:	dc.w	$2222
.x:	dc.w	$3333
	dc.w	Lp.x	; REF
	endm
	mac
	dc.w	$4444
