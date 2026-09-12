; label in a `rept` inside a macro body, read after the expansion
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	rept	1
Lr:	dc.w	$2222
	endm
	endm
	mac
	dc.w	Lr	; REF
	dc.w	$4444
