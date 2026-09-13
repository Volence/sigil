	cpu 68000
	padding off
	org 0
fm function n,"n"
	dc.b fm(12),$EE
	dc.b fm($10),$EE
	dc.b fm(1+2),$EE
	dc.b fm(-5),$EE
	dc.b fm(n2),$EE
	dc.b fm(%101),$EE
	dc.b fm(255),$EE
n2 equ $1F
	end
