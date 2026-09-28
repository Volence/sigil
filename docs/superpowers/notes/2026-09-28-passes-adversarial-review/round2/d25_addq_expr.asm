	cpu 68000
	nop
	addq.w #Fwd+20,d0
X:	nop
	dc.w X
Fwd	equ 1
