	cpu 68000
	padding off
	org 0
Ver set 2
	pushv ,Ver
Ver set 4
	dc.b Ver
	popv ,Ver
	dc.b Ver
	dc.b $EE
	end
