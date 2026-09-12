	cpu 68000
	padding off
	org 0
	dc.l int()
	dc.l abs()
	dc.b abs()
	dc.l lastbit()
	dc.l abs( )
	dc.l abs(())
	move.l #abs(),d0
X equ abs()
	dc.l X
	dc.l abs()+5
	dc.b int( )
	if abs()=0
	dc.b 1
	else
	dc.b 2
	endif
	dc.l INT(sin())
	dc.l INT(cos())
	dc.l INT(tan())
	dc.l INT(atan())
	dc.l INT(asin())
	dc.l INT(acos())
	dc.l INT(sinh())
	dc.l INT(cosh())
	dc.l INT(tanh())
	dc.l INT(asinh())
	dc.l INT(atanh())
	dc.l INT(sqrt())
	dc.l INT(exp())
	dc.b $EE
	end
