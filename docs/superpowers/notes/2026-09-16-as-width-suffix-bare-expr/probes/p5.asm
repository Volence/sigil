	cpu	68000
	padding	off
	supmode	on
	org	0
	move.w	(pc),d0
	move.w	(sr),d0
	move.w	(usp),d0
	move.w	(d0),d0
