	cpu 68000
	padding off
	org 0
A := 1
	popv ,A
	dc.b A
	dc.b $EE
	end
