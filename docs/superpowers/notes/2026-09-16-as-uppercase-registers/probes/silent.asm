	cpu	68000
	org	$1000
A0:	equ	$1234
SP:	equ	$5678
D0:	equ	$9abc
	move.w	A0,d0
	move.w	SP,d0
	move.w	D0,d1
	end
