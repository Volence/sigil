; body `if 1` / `-` / `endif`; `-` after the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	if	1
-	dc.w	$2222
	endif
	endm
	mac
	dc.w	-	; REF
	dc.w	$4444
