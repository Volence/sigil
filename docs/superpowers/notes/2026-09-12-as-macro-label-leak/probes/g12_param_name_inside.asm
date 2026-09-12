; argument-text label read INSIDE the body (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	nm
nm:	dc.w	$2222
	dc.w	nm	; REF
	endm
	mac	Foo
	mac	Bar
	dc.w	$4444
