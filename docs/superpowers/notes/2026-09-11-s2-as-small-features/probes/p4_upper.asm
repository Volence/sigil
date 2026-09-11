	cpu 68000
	padding off
	org 0
Ver := 2
	PUSHV ,Ver
Ver := 4
	dc.b Ver
	POPV ,Ver
	dc.b Ver
	dc.b $EE
	end
