; outer calls inner, then `.y:` in the outer body; read `Inner.y` after outer
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
	endm
	outer
	dc.w	Inner.y	; REF
	dc.w	$4444
