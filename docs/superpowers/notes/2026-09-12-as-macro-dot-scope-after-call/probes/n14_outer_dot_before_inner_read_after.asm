; outer body `.lp:`, calls inner (writes `Inner:`), then reads `.lp` in its body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
inner	macro
Inner:	dc.w	$2222
	endm
outer	macro
.lp:	dc.w	$3333
	inner
	dc.w	.lp	; REF
	endm
	outer
	dc.w	$4444
