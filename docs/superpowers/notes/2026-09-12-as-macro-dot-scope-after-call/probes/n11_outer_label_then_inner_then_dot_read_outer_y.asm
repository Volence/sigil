; outer writes `Outer:`, calls inner, then `.y:`; the outer body reads `Outer.y`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Inner:	dc.w	$2222
	endm
outer	macro
Outer:	dc.w	$3333
	inner
.y:	dc.w	$3334
	dc.w	Outer.y	; REF
	endm
	outer
	dc.w	$4444
