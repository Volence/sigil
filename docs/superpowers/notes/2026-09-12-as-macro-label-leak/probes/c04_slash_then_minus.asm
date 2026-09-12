; column-1 `/` in body, `-` referenced after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
/	dc.w	$2222
	endm
	mac
	dc.w	-	; REF
	dc.w	$4444
