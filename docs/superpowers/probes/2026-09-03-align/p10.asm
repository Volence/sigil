	cpu	68000
	padding	off
	phase	$FFFF0000
	ds.b	$B02B
	align	2
M:	dc.w	M&$FFFF
	dephase
