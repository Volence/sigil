; body `La:` `.x:` `Lb:` `.x:`, read `.x` inside after each
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
La:	dc.w	$2222
.x:	dc.w	.x	; REF
Lb:	dc.w	$3333
.x:	dc.w	.x
	endm
	mac
	dc.w	$4444
