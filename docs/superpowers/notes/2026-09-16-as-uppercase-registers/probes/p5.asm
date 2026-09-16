	cpu	68000
	org	$1000
A0:	equ	$1234
	move.w	(A0).w,d0
	end
