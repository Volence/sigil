	cpu 68000
	if Sel=1
	include "inc_a.inc"
	else
	include "inc_b.inc"
	endif
L:	dc.w L
Sel equ 2
