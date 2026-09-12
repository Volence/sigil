	cpu 68000
	padding off
	org 0
Q equ 'a'
	dc.l lastbit(Q)
	dc.b $EE
	end
