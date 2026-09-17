	cpu 68000
	padding off
	org 0
	move.w	(OddData).l,d0
	jsr	(OddData).l
	lea	(OddData).l,a1
	move.b	(OddData).l,d0
	dc.b	0
OddData:
	dc.b	1
