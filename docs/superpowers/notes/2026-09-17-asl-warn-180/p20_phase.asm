	cpu 68000
	padding off
	org 0
	phase $FF0001
plbl:
	dephase
	move.w	(plbl).l,d0
	move.w	(plbl&$FFFF).w,d0
