	cpu	68000
	padding	off
	phase	$FFFF0000
	ds.b	$B02A
	align	256
L:	dc.w	L&$FFFF
	dephase
