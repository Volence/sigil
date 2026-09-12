; plain label in a file-level `rept 2` body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	2
Lr:	dc.w	$2222
	endm
	dc.w	Lr	; REF
	dc.w	$4444
