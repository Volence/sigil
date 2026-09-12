	cpu 68000
	padding off
	org 0
signPrefix function number,substr("-",0,-sgn(number))
	dc.b signPrefix(-5)
	dc.b signPrefix(0)
	dc.b signPrefix(-$123),"$"
	dc.b substr("-",0,-sgn(-5)),"x"
	dc.b $EE
	end
