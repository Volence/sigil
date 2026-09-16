	cpu	68000
	padding	off
	supmode	on
	org	0
a0:	equ	$1234
sp:	equ	$1234
	move.w	(a0),d0
	move.w	(sp),d0
