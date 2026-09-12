; colon-less column-0 label alone on its line in body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp
	dc.w	$2222
	endm
	mac
	dc.w	Lp	; REF
	dc.w	$4444
