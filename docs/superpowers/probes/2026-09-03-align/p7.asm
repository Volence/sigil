	cpu	68000
	padding	off
	phase	$FFFF0000
	align	256
X:	dc.w	X
	dephase
	org	$FFFFB02A
	align	256
Y:	dc.w	Y
	org	$FFFFB02A
	align	100
Z:	dc.w	Z
	org	$0000B02A
	align	100
W:	dc.w	W
