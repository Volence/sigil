; {GLOBALSYMBOLS} body label, macro invoked twice (collision?)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{GLOBALSYMBOLS}
Lp:	dc.w	$2222
	endm
	mac
	mac
	dc.w	Lp	; REF
	dc.w	$4444
