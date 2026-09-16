	cpu	68000
A0:	equ	$1234
	dc.w	A0+1
	move.w	#A0+1,d0
	end
