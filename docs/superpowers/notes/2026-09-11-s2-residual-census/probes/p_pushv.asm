	cpu 68000
	padding off
	org 0
Ver := 2
	dc.b Ver
	pushv ,Ver
Ver := 4
	dc.b Ver
	popv ,Ver
	dc.b Ver
	end
