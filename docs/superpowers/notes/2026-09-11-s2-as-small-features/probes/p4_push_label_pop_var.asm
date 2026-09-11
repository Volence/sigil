	cpu 68000
	padding off
	org 0
	dc.b 0
Lbl:	dc.b 1
A := 7
	pushv ,Lbl
	popv ,A
	dc.b A
	dc.b $EE
	end
