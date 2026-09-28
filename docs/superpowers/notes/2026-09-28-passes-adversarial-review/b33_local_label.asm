	cpu 68000
Outer:
	bra.w .x
	if N=1
	dc.w 0
	endif
.x:	nop
	dc.w .x
N equ 1
