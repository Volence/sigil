	cpu 68000
	padding off
	org 0
odd1 equ 1
odd3 = 3
even2 equ 2
	move.w	(odd1).w,d0
	move.w	(odd3).w,d0
	move.w	(even2).w,d0
	move.w	(even2+1).w,d0
	move.w	(odd1+1).w,d0
	move.w	(fwd).w,d0
	move.w	(fwdeven).w,d0
fwd equ 5
fwdeven equ 6
