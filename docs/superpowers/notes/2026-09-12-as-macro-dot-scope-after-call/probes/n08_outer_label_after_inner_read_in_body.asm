; outer calls inner, then `.y:` in the outer body read as `.y` there
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
	dc.w	.y	; REF
	endm
	outer
	dc.w	$4444
