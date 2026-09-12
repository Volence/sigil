; plain outer calls {GLOBALSYMBOLS} inner writing `.dl:` and reading `.dl` itself
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro	{GLOBALSYMBOLS}
.dl:	dc.w	$2222
	dc.w	.dl	; REF
	endm
outer	macro
	inner
	endm
	outer
	outer
	dc.w	$4444
