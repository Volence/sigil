; body `Lab{n}:` invoked twice with the same n
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	n
Lab{n}:	dc.w	$2222
	endm
	mac	3
	mac	3
	dc.w	$3333
	dc.w	$4444
