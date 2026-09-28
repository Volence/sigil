	cpu 68000
	nop
	move.b d0,Fwd(a1)
After:
	ifndef Seen
Seen = After
	endif
	dc.w Seen
Fwd	equ 0
