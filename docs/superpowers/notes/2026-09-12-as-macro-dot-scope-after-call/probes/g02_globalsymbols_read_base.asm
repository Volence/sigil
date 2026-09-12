; `{GLOBALSYMBOLS}` body `Inner:`; after `.b := 2` read `Base.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro	{GLOBALSYMBOLS}
Inner:	dc.w	$2222
	endm
	mac
.b	:=	2
	dc.w	Base.b	; REF
	dc.w	$4444
