; plain outer invoked twice, each calls {GLOBALSYMBOLS} inner (defines Lg)
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
	outer
	dc.w	$3333
	dc.w	$4444
