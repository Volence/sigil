; outer calls inner (writes `Inner:`) then writes `Outer2:`; read `Inner.b`
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
Outer2:	dc.w	$3333
	endm
	outer
.b	:=	2
	dc.w	Inner.b	; REF
	dc.w	$4444
