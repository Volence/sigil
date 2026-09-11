	cpu 68000
	padding off
	org 0
n	equ 5
	message "<\{n+\x31}>"
	dc.b $EE
	end
