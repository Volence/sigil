; plain outer calls {GLOBALSYMBOLS} inner (defines Lg); outer reads Lg after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
inner	macro	{GLOBALSYMBOLS}
Lg:	dc.w	$2222
	endm
outer	macro
	inner
	dc.w	Lg	; REF
	endm
	outer
	dc.w	$4444
