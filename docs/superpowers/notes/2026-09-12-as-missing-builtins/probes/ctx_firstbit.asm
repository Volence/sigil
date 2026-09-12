	cpu 68000
	padding off
	org 0
	dc.b firstbit(FwdL)
	ds.b $3F
FwdL:
	dc.b firstbit(12)
	dc.b FIRSTBIT(12)
	dc.b FirstBit(12)
	dc.b firstbit( 12 )
	dc.b firstbit((12))
	dc.b firstbit(Later)
X equ firstbit(12)
	dc.b X
Y set firstbit(12)
	dc.b Y
	if firstbit(12)=firstbit(12)+1
	dc.b 2
	else
	dc.b 1
	endif
	move.l #firstbit(12),d0
	move.w #firstbit(12)<<2,d1
	dc.b firstbit(12)+1
	dc.b firstbit(firstbit(12))
	dc.b "\{firstbit(12)}"
	dc.b firstbit()
	dc.b firstbit( )
	dc.b firstbit(())
Later equ 12
	dc.b $EE
	end
