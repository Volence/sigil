	cpu	68000
	padding	off
	supmode	on
	org	0
Tbl:	dc.w	1,2
	move.w	(Tbl,pc),d1
	move.w	(FwdLab),d0
	move.w	FwdLab,d0
	move.l	(FwdLab),d1
FwdLab:	dc.w	0
