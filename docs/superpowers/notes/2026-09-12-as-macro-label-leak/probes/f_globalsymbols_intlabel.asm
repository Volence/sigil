; {INTLABEL,GLOBALSYMBOLS}: `__LABEL___Blocks:` read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	{INTLABEL},{GLOBALSYMBOLS}
__LABEL___Blocks:	dc.w	$2222
	endm
Aint:	mac
	dc.w	Aint_Blocks	; REF
	dc.w	$4444
