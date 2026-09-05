	cpu	68000
	if	MOMPASS > 1
	dc.l	$11111111
	endif
	dc.l	$22222222
