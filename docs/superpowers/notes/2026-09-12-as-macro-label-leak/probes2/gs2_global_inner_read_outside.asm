; plain outer calls {GLOBALSYMBOLS} inner (defines Lg); read at file level
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro	{GLOBALSYMBOLS}
Lg:	dc.w	$2222
	endm
outer	macro
	inner
	endm
	outer
	dc.w	Lg	; REF
	dc.w	$4444
