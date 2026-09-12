; plain label in a file-level `irpc` body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	irpc	c,"a"
Lc:	dc.w	$2222
	endm
	dc.w	Lc	; REF
	dc.w	$4444
