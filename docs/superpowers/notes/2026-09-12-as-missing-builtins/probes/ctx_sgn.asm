	cpu 68000
	padding off
	org 0
	dc.b sgn(FwdL)
	ds.b $3F
FwdL:
	dc.b sgn(-5)
	dc.b SGN(-5)
	dc.b Sgn(-5)
	dc.b sgn( -5 )
	dc.b sgn((-5))
	dc.b sgn(Later)
X equ sgn(-5)
	dc.b X
Y set sgn(-5)
	dc.b Y
	if sgn(-5)=sgn(-5)+1
	dc.b 2
	else
	dc.b 1
	endif
	move.l #sgn(-5),d0
	move.w #sgn(-5)<<2,d1
	dc.b sgn(-5)+1
	dc.b sgn(sgn(-5))
	dc.b "\{sgn(-5)}"
	dc.b sgn()
	dc.b sgn( )
	dc.b sgn(())
Later equ -5
	dc.b $EE
	end
