	cpu	68000
	padding	off
; bare labels only: read the listing PC column, no dc.x to overflow.
	org	$FFFFB001
	align	256
A1:
	org	$FFFFB002
	align	256
A2:
	org	$FFFFB000
	align	256
A0:
	org	$0000B001
	align	256
B1:
	org	$80000000
	align	256
C0:
	org	$7FFFB02A
	align	256
D0:
	org	$FFFFB02A
	align	100
E0:
	org	$FFFFFF00
	align	256
F0:
	org	$FFFFFF01
	align	256
G0:
