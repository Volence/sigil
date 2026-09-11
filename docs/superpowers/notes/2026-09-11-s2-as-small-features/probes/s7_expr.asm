	cpu 68000
	padding off
	org 0
A:	dc.b 1
	shared A+1
	dc.b $EE
	end
