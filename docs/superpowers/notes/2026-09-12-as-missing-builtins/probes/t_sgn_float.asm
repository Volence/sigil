	cpu 68000
	padding off
	org 0
	dc.l sgn(0.0)
	dc.l sgn(-0.0)
	dc.l sgn(0.5)
	dc.l sgn(-0.5)
	dc.l sgn(1.0)
	dc.l sgn(-1.0)
	dc.l sgn(2.5)
	dc.l sgn(-2.5)
	dc.l sgn(1000000.0)
	dc.l sgn(-1000000.0)
	dc.l sgn(0.000001)
	dc.l sgn(-0.000001)
	dc.b $EE
	end
