	cpu	68000
om	macro	address
	if	address < *
	dc.l	$11111111
	elseif	address > *
	dc.l	$33333333
	endif
	endm
	dc.l	$22222222
	om	$100
