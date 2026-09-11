	cpu 68000
	padding off
	org 0
Ver := 2
	pushv ,Ver
Ver := 4
	bra.w Fwd
	dc.b Ver
	popv ,Ver
Fwd:	dc.b Ver
	dc.b $EE
	end
