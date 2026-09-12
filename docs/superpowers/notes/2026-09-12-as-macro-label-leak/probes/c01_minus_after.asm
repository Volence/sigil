; column-1 `-` defined in body, `-` referenced after the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
-	dc.w	$2222
	endm
	mac
	dc.w	-	; REF
	dc.w	$4444
