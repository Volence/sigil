; `{GLOBALSYMBOLS}` outer calls plain inner writing `Inner:`; read `Inner.b` after outer
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Inner:	dc.w	$2222
	endm
outer	macro	{GLOBALSYMBOLS}
	inner
	endm
	outer
.b	:=	2
	dc.w	Inner.b	; REF
	dc.w	$4444
