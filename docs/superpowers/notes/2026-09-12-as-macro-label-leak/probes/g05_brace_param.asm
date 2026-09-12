; body `Lab{n}:` from parameter `n`, read `Lab3` after
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro	n
Lab{n}:	dc.w	$2222
	endm
	mac	3
	dc.w	Lab3	; REF
	dc.w	$4444
