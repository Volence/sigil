	cpu 68000
	org 0
M	macro a,b
	dc.b a,b
	endm
	M a=1,2
	end
