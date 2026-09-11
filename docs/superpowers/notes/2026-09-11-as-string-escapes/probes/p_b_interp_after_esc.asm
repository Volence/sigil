	cpu 68000
	padding off
	org 0
n	equ 5
	dc.b "\x41\{n}"
	dc.b $EE
	end
