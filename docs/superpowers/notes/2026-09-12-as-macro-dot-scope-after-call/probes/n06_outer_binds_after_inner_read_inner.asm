; outer calls inner, then `.v := 3` in the outer body; read `Inner.v` after
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
.v	:=	3
	endm
	outer
	dc.w	Inner.v	; REF
	dc.w	$4444
