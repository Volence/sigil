; body `Lab{n}:` read inside the body as `Lab{n}` (control)
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	n
Lab{n}:	dc.w	$2222
	dc.w	Lab{n}	; REF
	endm
	mac	3
	mac	4
	dc.w	$4444
