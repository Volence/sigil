	cpu 68000
	nop
	addq.w #Fwd-Fwd2,d0
X:	nop
	dc.w X
Fwd	equ 9
Fwd2	equ 0
