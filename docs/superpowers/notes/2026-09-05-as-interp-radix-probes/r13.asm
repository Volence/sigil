	cpu 68000
	padding off
	org 0
n	:= 42
s	:= "\{n}"
n	:= 255
	dc.b s
	dc.b $ff
m	equ 42
t	equ "\{m}"
	dc.b t
	dc.b $fe
	end
