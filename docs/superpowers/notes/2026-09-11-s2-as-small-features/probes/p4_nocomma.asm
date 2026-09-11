	cpu 68000
	padding off
	org 0
Ver := 2
	pushv Ver
	popv Ver
	dc.b Ver
	dc.b $EE
	end
