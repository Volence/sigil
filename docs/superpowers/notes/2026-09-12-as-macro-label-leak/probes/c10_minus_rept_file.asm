; column-1 `-` inside a file-level `rept 1` body, `-` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	1
-	dc.w	$2222
	endm
	dc.w	-	; REF
	dc.w	$4444
