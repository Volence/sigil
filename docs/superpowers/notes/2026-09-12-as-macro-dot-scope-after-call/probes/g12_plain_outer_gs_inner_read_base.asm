; plain outer calls `{GLOBALSYMBOLS}` inner writing `Inner:`; read `Base.b` after outer
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro	{GLOBALSYMBOLS}
Inner:	dc.w	$2222
	endm
outer	macro
	inner
	endm
	outer
.b	:=	2
	dc.w	Base.b	; REF
	dc.w	$4444
