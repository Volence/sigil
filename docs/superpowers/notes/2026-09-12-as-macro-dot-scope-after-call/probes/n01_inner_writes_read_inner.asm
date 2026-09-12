; outer calls inner, inner writes `Inner:`; after outer `.b := 2` read `Inner.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Inner:	dc.w	$2222
	endm
outer	macro
	inner
	endm
	outer
.b	:=	2
	dc.w	Inner.b	; REF
	dc.w	$4444
