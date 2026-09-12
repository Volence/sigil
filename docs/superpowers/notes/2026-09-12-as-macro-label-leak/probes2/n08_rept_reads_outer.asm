; outer `-` before; `rept 2` body references `-`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$3333
	rept	2
	dc.w	-	; REF
	endm
	dc.w	$4444
