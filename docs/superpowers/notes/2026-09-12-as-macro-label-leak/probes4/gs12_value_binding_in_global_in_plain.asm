; plain outer (under Base) calls {GLOBALSYMBOLS} inner writing `.v := 7`; read `Base.v` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro	{GLOBALSYMBOLS}
.v	:=	7
	endm
outer	macro
	inner
	endm
	outer
	dc.w	Base.v	; REF
	dc.w	$4444
