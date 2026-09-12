; argument-text label, macro invoked twice with the SAME name
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	nm
nm:	dc.w	$2222
	endm
	mac	Foo
	mac	Foo
	dc.w	$3333
	dc.w	$4444
