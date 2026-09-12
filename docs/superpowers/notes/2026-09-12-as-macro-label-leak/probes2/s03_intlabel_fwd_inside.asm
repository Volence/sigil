; {INTLABEL} body `dc.w __LABEL___B` then `__LABEL___B:`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{INTLABEL}
	dc.w	__LABEL___B	; REF
__LABEL___B:	dc.w	$2222
	endm
Aint:	mac
Bint:	mac
	dc.w	$4444
