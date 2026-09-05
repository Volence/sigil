	cpu 68000
	padding off
	org 0
n	:= 42
name_{"\{n}"}	equ $55
	message "hexname=\{name_2A}"
	dc.b $11
	end
