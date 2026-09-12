; plain inner macro's label, inner called from a {GLOBALSYMBOLS} outer, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro
Ln:	dc.w	$2222
	endm
outer	macro	{GLOBALSYMBOLS}
	inner
	endm
	outer
	dc.w	Ln	; REF
	dc.w	$4444
