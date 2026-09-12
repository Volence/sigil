; outer body `Lp:` and `.x:`, then calls inner, which reads `.x`
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
.x:	dc.w	$3334
	inner
	endm
	outer
	dc.w	$4444
