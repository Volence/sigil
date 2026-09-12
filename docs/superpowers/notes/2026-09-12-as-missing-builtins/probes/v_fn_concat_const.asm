	cpu 68000
	padding off
	org 0
f function number,"a"+"b"
	dc.b f(1)
	dc.b $EE
	end
