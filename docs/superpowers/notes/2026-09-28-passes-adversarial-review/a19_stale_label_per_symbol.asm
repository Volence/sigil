	cpu 68000
	nop
	move.b d0,Fwd-2(a1)
After:
	if After=4
StaleLab:
	endif
	dc.w StaleLab
Fwd	equ $40
