	cpu	68000
	padding	off
	phase	$B000
	ds.b	$2A
	align	2
M:	dc.w	M
	dephase
