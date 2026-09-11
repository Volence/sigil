	cpu 68000
	padding off
	org 0
A := 1
B := 2
	pushv ,A,B
A := 3
B := 4
	popv ,A
	dc.b A,B
	popv ,A
	dc.b A,B
	dc.b $EE
	end
