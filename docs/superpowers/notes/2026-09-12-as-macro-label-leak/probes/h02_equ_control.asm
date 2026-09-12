; `equ` in body, read after (control: global)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
Eq	equ	$123
	endm
	mac
	dc.w	Eq	; REF
	dc.w	$4444
