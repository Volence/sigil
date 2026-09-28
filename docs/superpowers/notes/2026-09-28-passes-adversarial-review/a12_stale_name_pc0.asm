	cpu 68000
	move.b d0,Fwd(a1)
After:
	if After=2
Stale = 1
	endif
	ifdef Stale
	dc.b $AA
	endif
	dc.b 0
Fwd	equ $40
