	cpu 68000
	padding off
	org 0
signPrefix function number,substr("-",0,-sgn(number))
	dc.b "<",signPrefix(5),">"
	dc.b $EE
	end
