	cpu 68000
	padding off
	org 0
f function x,y,x+y
	dc.b f((),1)
	dc.b $EE
	end
