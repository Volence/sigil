	cpu 68000
	padding off
	org 0
	jmp	(a0)
	jmp	1(a0)
	move.w	(a1),d0
	move.w	(sp)+,d0
	move.w	-(sp),d0
