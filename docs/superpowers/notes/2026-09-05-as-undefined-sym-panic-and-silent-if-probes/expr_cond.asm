	cpu	68000
K	equ	3
J	equ	4
	if	(K*2)=6&&(J<>3)
	dc.l	$11111111
	endif
	dc.l	$22222222
