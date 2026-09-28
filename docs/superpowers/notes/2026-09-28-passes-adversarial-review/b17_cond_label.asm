	cpu 68000
	if X<>0
L1:	dc.b 1,2
	else
L1:	dc.b 3
	endif
	dc.w L1
X equ L2
L2:	dc.w L2
