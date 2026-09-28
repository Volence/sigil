	cpu 68000
	rept N
	dc.b $11
	endm
L:	dc.w L
N equ 3
