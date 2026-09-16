	cpu	68000
	org	$1000
A0:	equ	$1234
SP:	equ	$5678
D0:	equ	$9abc
	move.w	(A0),d0
	move.w	(a0),d0
	move.w	A0,d0
	move.w	a0,d0
	move.w	(SP),d0
	move.w	SP,d0
	move.w	D0,d1
	move.w	(A0)+,d0
	move.w	-(A0),d0
	move.w	(4,A0),d0
	move.w	4(A0),d0
	move.w	(4,A0,D1.W),d0
	movem.w	D0/A0,-(sp)
	lea	(A0),A1
	end
