	cpu 68000
	padding off
	org 0
	move.w	oddlbl(pc),d0
	lea	oddlbl(pc),a0
	jmp	oddlbl(pc)
	jsr	oddlbl(pc)
	move.b	oddlbl(pc),d0
	move.w	oddlbl(pc,d0.w),d0
	move.w	(oddlbl).w,d0
	move.w	(oddlbl).l,d0
	dc.b	0
oddlbl:
	dc.b	0
