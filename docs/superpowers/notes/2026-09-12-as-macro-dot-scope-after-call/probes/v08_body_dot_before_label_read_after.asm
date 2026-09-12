; body `.lp:`, then `Inner:`, then reads `.lp`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
.lp:	dc.w	$2222
Inner:	dc.w	$2223
	dc.w	.lp	; REF
	endm
	mac
	dc.w	$4444
