	cpu 68000
	padding off
	dc.w (1.5<2)+Lbl
	move.l #INT(3.7),d0
	dc.l INT(.5+3),INT(1.)
Lbl:
	dc.b 0
	end
