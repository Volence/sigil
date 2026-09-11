	cpu 68000
	padding off
	org 0
A := 1
	pushv ,A
A := 2
	pushv s1,A
A := 3
	popv ,A
	dc.b A
	popv s1,A
	dc.b A
	dc.b $EE
	end
