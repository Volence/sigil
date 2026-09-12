	cpu 68000
	padding off
	org 0
f function number,substr("-",0,-sgn(number))+"$\{abs(number)}"
S set f(-5)
	dc.b S
	dc.b $EE
	end
