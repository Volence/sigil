; outer body `Lp:`, calls inner, which reads `.x`; outer defines `.x:` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
	dc.w	.x	; REF
	endm
outer	macro
Lp:	dc.w	$3333
	inner
.x:	dc.w	$3334
	endm
	outer
	dc.w	$4444
