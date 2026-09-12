; `enum` member read inside the body (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
	enum	Ea=5,Eb
	dc.w	Eb	; REF
	endm
	mac
	dc.w	$4444
