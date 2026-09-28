	cpu 68000
I set 0
	while I<N
	dc.b I
I set I+1
	endw
L:	dc.w L
N equ 3
