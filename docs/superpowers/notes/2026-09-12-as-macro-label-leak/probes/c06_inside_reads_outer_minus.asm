; outer `-` before the call; the body references `-`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
-	dc.w	$3333
mac	macro
	dc.w	-	; REF
	endm
	mac
	dc.w	$4444
