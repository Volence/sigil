; plain colon label in body, read after the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Lp:	dc.w	$2222
	endm
	mac
	dc.w	Lp	; REF
	dc.w	$4444
