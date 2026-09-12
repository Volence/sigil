; {INTLABEL} body `__LABEL___Blocks:`, read `Aint_Blocks` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{INTLABEL}
__LABEL___Blocks:	dc.w	$2222
	endm
Aint:	mac
	dc.w	Aint_Blocks	; REF
	dc.w	$4444
