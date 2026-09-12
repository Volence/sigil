; `rept 2` body `-` then `dc.w -`
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
	rept	2
-	dc.w	$2222
	dc.w	-	; REF
	endm
	dc.w	$4444
