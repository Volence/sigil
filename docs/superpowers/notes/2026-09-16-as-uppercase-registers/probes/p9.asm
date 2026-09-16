	cpu	68000
A0:	equ	$1234
SP:	equ	$5678
D0:	equ	$9abc
SR:	equ	$4444
CCR:	equ	$2222
USP:	equ	$3333
	move.w	(A0),d0
	move.w	A0,d0
	move.w	(SP),d0
	move.w	SP,d0
	move.w	Sp,d0
	move.w	D0,d1
	move.w	(A0)+,d0
	move.w	-(A0),d0
	move.w	(4,A0),d0
	move.w	4(A0),d0
	move.w	(4,A0,D1.W),d0
	move.w	(A0,D1),d0
	move.w	(A0,D1.L),d0
	movem.w	D0/A0,-(sp)
	lea	(A0),A1
	move.w	SR,d0
	move.b	d0,CCR
	move.l	a0,USP
	end
