	cpu 68000
	nop
	move.b d0,Fwd(a1)
After:
	if After=6
StaleLab:
	endif
	dc.w StaleLab
Fwd	equ 0
