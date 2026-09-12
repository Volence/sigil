; outer calls inner (writes `Inner:`), then `bra.s .y` forward to its own `.y:`
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
	bra.s	.y	; REF
	dc.w	$3333
.y:	dc.w	$3334
	endm
	outer
	dc.w	$4444
