; `enum` members declared in a body, read after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	enum	Ea=5,Eb
	endm
	mac
	dc.w	Eb	; REF
	dc.w	$4444
