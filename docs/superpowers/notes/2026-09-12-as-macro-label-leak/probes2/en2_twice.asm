; `enum` in a body, macro invoked twice
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	enum	Ea=5,Eb
	endm
	mac
	mac
	dc.w	$3333
	dc.w	$4444
