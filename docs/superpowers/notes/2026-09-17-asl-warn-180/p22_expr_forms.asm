	cpu 68000
	padding off
	org 0
base equ $FFFF8000
	move.w	(base+1).w,d0
	move.w	(base+2).w,d0
	move.w	((3*5)).w,d0
	move.w	(15/2).w,d0
	move.w	($FF0000|1).l,d0
