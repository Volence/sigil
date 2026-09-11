	cpu 68000
	padding off
	org 0
A := 1
	pushv S1,A
A := 2
	popv s1,A
	dc.b A
	dc.b $EE
	end
