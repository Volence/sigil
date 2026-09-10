	cpu 68000
	org 0
M	macro a,b
	dc.b a,b
	endm
	M 1,b=2
	end
