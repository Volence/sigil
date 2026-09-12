; body `Lp:` `.x:`, macro invoked twice
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp:	dc.w	$2222
.x:	dc.w	$3333
	endm
	mac
	mac
	dc.w	$5555
	dc.w	$4444
