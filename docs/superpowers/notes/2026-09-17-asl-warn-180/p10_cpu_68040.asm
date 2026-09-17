	cpu 68040
	padding off
	org 0
	move.w	(1).w,d0
	move.l	d0,(3).w
	jmp	(1).w
