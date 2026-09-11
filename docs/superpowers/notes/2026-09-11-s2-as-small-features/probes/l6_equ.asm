	cpu 68000
	padding off
	org 0
X equ lastbit(5)
	dc.b X
	dc.b $EE
	end
