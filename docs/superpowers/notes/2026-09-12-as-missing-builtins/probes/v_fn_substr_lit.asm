	cpu 68000
	padding off
	org 0
f function number,substr("-",0,1)
	dc.b f(-5)
	dc.b $EE
	end
