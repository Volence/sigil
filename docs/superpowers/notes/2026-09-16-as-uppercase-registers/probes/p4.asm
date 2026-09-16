	cpu	68000
	org	$1000
D0:	equ	$1234
	move.w	(D0),d1
	end
