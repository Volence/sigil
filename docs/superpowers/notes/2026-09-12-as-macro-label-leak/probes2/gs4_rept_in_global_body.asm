; {GLOBALSYMBOLS} body `rept 1` with a label, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{GLOBALSYMBOLS}
	rept	1
Lr:	dc.w	$2222
	endm
	endm
	mac
	dc.w	Lr	; REF
	dc.w	$4444
