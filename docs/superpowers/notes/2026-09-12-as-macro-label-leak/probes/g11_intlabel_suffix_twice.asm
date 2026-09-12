; {INTLABEL} `__LABEL___Blocks:` invoked twice under the SAME label? (control on collision)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{INTLABEL}
__LABEL___Blocks:	dc.w	$2222
	endm
Aint:	mac
Bint:	mac
	dc.w	$3333
	dc.w	$4444
