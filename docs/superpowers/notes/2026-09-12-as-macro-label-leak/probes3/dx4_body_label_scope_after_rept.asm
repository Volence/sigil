; file-level `rept 1` body `Lr:`; after the loop `.x:` and `dc.w .x`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	1
Lr:	dc.w	$2222
	endm
.x:	dc.w	$3333
	dc.w	.x	; REF
	dc.w	$4444
