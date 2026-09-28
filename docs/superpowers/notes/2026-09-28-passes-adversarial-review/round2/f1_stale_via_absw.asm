	cpu 68000
	phase $9000
	move.w Fwd,d0
After:
	if After=$9006
StaleLab:
	endif
	dc.w StaleLab
Fwd	equ $100
