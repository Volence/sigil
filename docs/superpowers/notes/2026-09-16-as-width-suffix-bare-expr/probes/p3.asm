	cpu	68000
	padding	off
	supmode	on
A0:	equ	$1234
	org	0
	move.w	A0,d0
	move.w	a0,d0
	move.w	(SP),d0
	move.w	D0,d1
