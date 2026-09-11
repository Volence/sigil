	cpu 68000
	padding off
	org 0
n	equ 5
s	:= "\x41\{n}\x42"
	dc.b s
	dc.b strlen(s)
	dc.b $EE
	end
