	cpu	68000
	org	$1000
A0:	equ	$1234
SP:	equ	$5678
D0:	equ	$9abc
	dc.w	A0
	dc.w	A0+1
	move.w	#A0+1,d0
	move.w	(A0+0),d0
	move.w	Sp,d0
	move.w	aO,d0
	end
