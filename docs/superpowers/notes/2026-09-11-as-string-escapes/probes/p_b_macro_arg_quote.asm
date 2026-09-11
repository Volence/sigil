	cpu 68000
	padding off
	org 0
m	macro a,b
	dc.b a
	dc.b b
	endm
	m "\"x,y",$11
	dc.b $EE
	end
