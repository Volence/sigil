	cpu	68000
	padding	off
	supmode	on
sr:	equ	$2000
ccr:	equ	$2002
usp:	equ	$2004
	org	0
	move.w	(sr),d0
	move.w	(ccr),d0
	move.w	(usp),d0
	move.w	(a7),d0
