; column-1 `-` in a `rept` inside a macro body, `-` after the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	rept	1
-	dc.w	$2222
	endm
	endm
	mac
	dc.w	-	; REF
	dc.w	$4444
