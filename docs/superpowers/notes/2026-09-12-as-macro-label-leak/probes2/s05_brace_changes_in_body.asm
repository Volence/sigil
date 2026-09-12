; body sets `cnt` then defines `Lab{cnt}:`, reads `Lab5` inside
	cpu	68000
	padding	off
	org	$100
	dc.w	$1111
mac	macro
cnt	set	5
Lab{cnt}:	dc.w	$2222
	dc.w	Lab5	; REF
	endm
	mac
	dc.w	$4444
