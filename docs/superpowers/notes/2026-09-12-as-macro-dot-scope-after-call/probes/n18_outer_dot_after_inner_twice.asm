; outer (calls inner, then `.y:`) expanded twice; the second reads `.y` (no collision)
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
.y:	dc.w	$3333
	dc.w	.y
	endm
	outer
	outer
	dc.w	$3335	; REF
	dc.w	$4444
