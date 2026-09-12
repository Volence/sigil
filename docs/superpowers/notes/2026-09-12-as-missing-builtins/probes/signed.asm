	cpu 68000
	padding off
	org 0
signedToString function number,substr("-",0,-sgn(number))+"$\{abs(number)}"
	dc.b signedToString(-5)
	dc.b signedToString(5)
	dc.b signedToString(0)
	dc.b signedToString(-$123)
	dc.b "\{sgn(-5)}"
	dc.b "\{bitcnt(-1)}"
	dc.b $EE
	end
