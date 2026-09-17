	cpu 68000
	padding off
	org 0
	if 0
	move.w	(1).w,d0
	endif
	if 1
	move.w	(3).w,d0
	endif
	if 0
	else
	move.w	(5).w,d0
	endif
