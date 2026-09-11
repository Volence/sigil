	cpu 68000
	padding off
	org 0
A := 7
	pushv ,A
	popv ,B
	dc.b A,B
	dc.b $EE
	end
