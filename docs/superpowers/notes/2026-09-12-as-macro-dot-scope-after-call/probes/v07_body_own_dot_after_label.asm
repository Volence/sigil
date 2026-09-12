; body `Inner:` then `.q:` and `dc.w .q` (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
Base:	dc.w	$5555
mac	macro
Inner:	dc.w	$2222
.q:	dc.w	$2223
	dc.w	.q	; REF
	endm
	mac
	dc.w	$4444
