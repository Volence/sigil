; {INTLABEL} body `__LABEL__:`, read `Aint` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{INTLABEL}
__LABEL__:	dc.w	$2222
	endm
Aint:	mac
	dc.w	Aint	; REF
	dc.w	$4444
