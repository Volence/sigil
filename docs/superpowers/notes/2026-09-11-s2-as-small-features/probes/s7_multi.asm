	cpu 68000
	padding off
	org 0
A:	dc.b 1
B:	dc.b 2
	shared A,B
	dc.b $EE
	end
