; {GLOBALSYMBOLS} body label read BEFORE the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{GLOBALSYMBOLS}
Lp:	dc.w	$2222
	endm
	dc.w	Lp	; REF
	mac
	dc.w	$4444
