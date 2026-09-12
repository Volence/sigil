; macro invoked twice, then read from outside
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp:	dc.w	$2222
	endm
	mac
	mac
	dc.w	Lp	; REF
	dc.w	$4444
