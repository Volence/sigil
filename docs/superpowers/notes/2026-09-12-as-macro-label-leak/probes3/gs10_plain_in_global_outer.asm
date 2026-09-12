; {GLOBALSYMBOLS} outer calls plain inner writing `Ln:`; outer reads Ln after inner
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
Ln:	dc.w	$2222
	endm
outer	macro	{GLOBALSYMBOLS}
	inner
	dc.w	Ln	; REF
	endm
	outer
	dc.w	$4444
