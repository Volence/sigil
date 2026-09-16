	cpu	68000
	padding	off
	supmode	on
	org	0
Tbl:	dc.w	1,2
	move.w	(Tbl,pc,d0.w),d1
	move.w	(Tbl,pc),d1
	jmp	(Tbl)
	jsr	(Tbl)
	jmp	(a0)
	jmp	(Tbl).l
	pea	(Tbl)
	movem.w	d0-d1,(Tbl)
	movem.w	(Tbl),d0-d1
	cmpi.w	#5,(Tbl)
	btst	#3,(Tbl)
	move.w	(Tbl),(Tbl)
	moveq	#0,d0
