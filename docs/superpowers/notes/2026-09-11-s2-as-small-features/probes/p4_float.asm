	cpu 68000
	padding off
	org 0
F := 1.5
	pushv ,F
F := 2.5
	popv ,F
	dc.l int(F*2)
	dc.b $EE
	end
