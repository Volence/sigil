; `.x:` under Base; body `Inner:` then reads `.x`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
.x:	dc.w	$6666
mac	macro
Inner:	dc.w	$2222
	dc.w	.x	; REF
	endm
	mac
	dc.w	$4444
