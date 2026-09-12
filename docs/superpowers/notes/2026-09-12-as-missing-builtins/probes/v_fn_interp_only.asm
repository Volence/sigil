	cpu 68000
	padding off
	org 0
f function number,"$\{abs(number)}"
	dc.b f(-5)
	dc.b $EE
	end
