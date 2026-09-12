; body `Lp:` `.x:`, then passes `.x` to a nested macro that emits it
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
emit	macro	v
	dc.w	v	; REF
	endm
mac	macro
Lp:	dc.w	$2222
.x:	dc.w	$3333
	emit	.x
	endm
	mac
	dc.w	$4444
