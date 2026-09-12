	cpu 68000
	padding off
	org 0
	dc.b bitpos(8)
	dc.b BITPOS(8)
	dc.b BitPos(8)
	dc.b bitpos( 8 )
	dc.b bitpos((8))
X equ bitpos(8)
	dc.b X
Y set bitpos(8)
	dc.b Y
	if bitpos(8)=bitpos(8)+1
	dc.b 2
	else
	dc.b 1
	endif
	move.l #bitpos(8),d0
	move.w #bitpos(8)<<2,d1
	dc.b bitpos(8)+1
	dc.b bitpos(bitpos(4))
	dc.b "\{bitpos(8)}"
	dc.b $EE
	end
