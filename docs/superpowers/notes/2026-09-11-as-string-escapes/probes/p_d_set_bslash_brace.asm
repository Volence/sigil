	cpu 68000
	padding off
	org 0
n	equ 5
s	:= "\\{n}"
	dc.b s
	dc.b $EE
	end
