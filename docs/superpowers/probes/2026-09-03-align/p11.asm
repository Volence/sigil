	cpu	68000
	padding	off
	phase	$FFFF0000
	ds.b	$B02A
	align	2
M:	dc.w	M&$FFFF
	dephase
