; {INTLABEL} body `__LABEL__Plc:` (not substituted), read `__LABEL__Plc` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{INTLABEL}
__LABEL__Plc:	dc.w	$2222
	endm
Aint:	mac
	dc.w	__LABEL__Plc	; REF
	dc.w	$4444
