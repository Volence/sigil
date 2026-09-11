	cpu 68000
	padding off
	org 0
	dc.b lastbit(1)
	dc.b lastbit(5)
	dc.b lastbit($80)
	dc.b lastbit($FFFEB)
	dc.l 2<<lastbit($FFFEB)
	dc.b $EE
	end
