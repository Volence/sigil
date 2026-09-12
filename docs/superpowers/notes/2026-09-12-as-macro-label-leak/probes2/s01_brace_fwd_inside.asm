; body `dc.w Lab{n}` then `Lab{n}:`, invoked with 3 and 4
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	n
	dc.w	Lab{n}	; REF
Lab{n}:	dc.w	$2222
	endm
	mac	3
	mac	4
	dc.w	$4444
