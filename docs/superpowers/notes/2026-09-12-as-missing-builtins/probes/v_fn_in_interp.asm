	cpu 68000
	padding off
	org 0
signedToString function number,substr("-",0,-sgn(number))+"$\{abs(number)}"
	message "A\{signedToString(-5)}B"
	dc.b "\{signedToString(-5)}",$EE
	dc.b "\{signedToString($123)}",$EE
	end
