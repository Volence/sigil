	cpu	68000
	padding	off
	supmode	on
	org	0
	move.w	(4,a0,d1.w),d0
	move.w	($1000).l,d0
V:	equ	$FFFFF728
	move.w	(V),d0
