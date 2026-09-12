; `{INTLABEL}` body `__LABEL__:`, called `Tbl mac`; after `.b := 2` read `Tbl.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro	{INTLABEL}
__LABEL__:	dc.w	$2222
	endm
Tbl	mac
.b	:=	2
	dc.w	Tbl.b	; REF
	dc.w	$4444
