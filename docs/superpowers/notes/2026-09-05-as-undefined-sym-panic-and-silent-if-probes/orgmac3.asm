	cpu	68000
notZ80	function	cpu,(cpu<>128)&&(cpu<>32988)
om	macro	address
	if	notZ80(MOMCPU)
		if	address < *
		dc.l	$11111111
		elseif	address > *
		dc.l	$33333333
		endif
	else
	dc.l	$44444444
	endif
	endm
A:
	dc.l	$22222222
B:
C:
	om	A+B-C
