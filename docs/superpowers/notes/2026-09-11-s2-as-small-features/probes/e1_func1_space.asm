	cpu 68000
	padding off
	org 0
f function x,x+1
	dc.b f( )
	dc.b $EE
	end
