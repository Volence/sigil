; inner writes `Lx label *`; outer body then `.y:` and reads `Lx.y` in its body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Lx	label	*
	endm
outer	macro
	inner
.y:	dc.w	$3333
	dc.w	Lx.y	; REF
	endm
	outer
	dc.w	$4444
