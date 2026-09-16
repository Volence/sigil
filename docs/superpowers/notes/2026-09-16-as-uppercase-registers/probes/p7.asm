	cpu	68000
	org	$1000
SR:	equ	$1234
CCR:	equ	$2222
USP:	equ	$3333
	move.w	SR,d0
	move.w	d0,SR
	move.b	CCR,d0
	move.b	d0,CCR
	move.l	USP,a0
	move.l	a0,USP
	move.w	(A0,D1),d0
	move.w	(A0,D1.L),d0
	end
