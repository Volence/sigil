; outer writes `Outer:` then calls a label-less inner; read `Outer.b`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
	dc.w	$2222
	endm
outer	macro
Outer:	dc.w	$3333
	inner
	endm
	outer
.b	:=	2
	dc.w	Outer.b	; REF
	dc.w	$4444
