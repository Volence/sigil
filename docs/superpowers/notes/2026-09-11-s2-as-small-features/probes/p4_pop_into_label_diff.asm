	cpu 68000
	padding off
	org 0
Lbl:	dc.b 1
A := 7
	pushv ,A
	popv ,Lbl
	dc.b Lbl
	dc.b $EE
	end
