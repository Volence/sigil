; label in a `rept` inside a macro body, read later in that body
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	rept	1
Lr:	dc.w	$2222
	endm
	dc.w	Lr	; REF
	endm
	mac
	dc.w	$4444
