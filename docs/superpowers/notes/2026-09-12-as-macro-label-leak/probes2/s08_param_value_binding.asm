; argument-text name bound with `equ` in the body, read after (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	nm
nm	equ	$123
	endm
	mac	Foo
	dc.w	Foo	; REF
	dc.w	$4444
