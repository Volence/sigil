	cpu 68000
	nop
	move.b d0,Fwd-2(a1)
After:
	if After=4
Stale = 1
	endif
	ifdef Stale
	dc.b $AA
	endif
	dc.b 0
Fwd	equ $40
