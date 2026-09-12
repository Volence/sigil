; {GLOBALSYMBOLS} body label read inside the body (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{GLOBALSYMBOLS}
Lp:	dc.w	$2222
	dc.w	Lp	; REF
	endm
	mac
	dc.w	$4444
