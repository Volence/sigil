	cpu 68000
	padding off
	org 0
rd	macro addr
	move.w	(addr).w,d0
	endm
	rd	1
	rd	2
	rd	3
