	cpu 68000
	padding off
	org 0
sv macro
	pushv ,Ver
Ver := 9
	dc.b Ver
	popv ,Ver
	endm
Ver := 3
	sv
	dc.b Ver
	dc.b $EE
	end
