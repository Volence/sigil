	cpu 68000
	padding off
	org 0
E equ 5
A := 7
	pushv ,A
	popv ,E
	dc.b E
	dc.b $EE
	end
