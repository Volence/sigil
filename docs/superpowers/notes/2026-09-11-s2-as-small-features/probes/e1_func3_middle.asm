	cpu 68000
	padding off
	org 0
f function x,y,z,x+y+z
	dc.b f(1,,2)
	dc.b $EE
	end
