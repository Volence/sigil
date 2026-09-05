	cpu	68000
	if	Later>0
	dc.l	$11111111
	endif
	dc.l	$22222222
Later:
	dc.l	$33333333
