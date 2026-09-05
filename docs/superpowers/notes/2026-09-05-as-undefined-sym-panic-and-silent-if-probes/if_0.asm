	cpu	68000
Nowhere	equ	0
	if	Nowhere
	dc.l	$11111111
	endif
	dc.l	$22222222
