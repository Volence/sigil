	cpu 68000
	padding off
	org 0
n	equ $12
	charset 'A',"\{n}"
	dc.b "AB"
	dc.b $EE
	end
