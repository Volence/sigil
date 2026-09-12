; plain colon label in a {EXPMACRO} body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{EXPMACRO}
Lp:	dc.w	$2222
	endm
	mac
	dc.w	Lp	; REF
	dc.w	$4444
