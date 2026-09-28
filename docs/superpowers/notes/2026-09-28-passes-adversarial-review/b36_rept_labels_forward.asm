	cpu 68000
	rept 3
	dc.w Q
	if N>0
	dc.b 1,1
	endif
Q:
	endm
N equ 1
