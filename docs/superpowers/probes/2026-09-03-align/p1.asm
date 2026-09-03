	cpu	68000
	padding	off
	phase	$B000
	ds.b	5
	align	256
L:	dc.w	L
	dephase
