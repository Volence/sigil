	cpu 68000
	padding off
	org 0
	dc.l sgn(-2.5)
	dc.b sgn(2.5)
	move.l #sgn(-0.5),d0
F set 2.5
	dc.l sgn(F)
	dc.l sgn(0.1-0.2)
	dc.l sgn(sqrt(2))
	dc.l INT(sgn(-2.5))
	dc.l sgn(-0.0)
	dc.b $EE
	end
