	cpu 68000
	nop
	move.b d0,Fwd(a1)
After:
	if After=6
StaleMac	macro
	dc.b $AA
	endm
	endif
	ifdef Fwd
	endif
	dc.b 0
Fwd	equ 0
	StaleMac
