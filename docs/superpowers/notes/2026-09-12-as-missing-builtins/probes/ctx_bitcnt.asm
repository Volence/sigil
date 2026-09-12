	cpu 68000
	padding off
	org 0
	dc.b bitcnt(FwdL)
	ds.b $3F
FwdL:
	dc.b bitcnt(7)
	dc.b BITCNT(7)
	dc.b BitCnt(7)
	dc.b bitcnt( 7 )
	dc.b bitcnt((7))
	dc.b bitcnt(Later)
X equ bitcnt(7)
	dc.b X
Y set bitcnt(7)
	dc.b Y
	if bitcnt(7)=bitcnt(7)+1
	dc.b 2
	else
	dc.b 1
	endif
	move.l #bitcnt(7),d0
	move.w #bitcnt(7)<<2,d1
	dc.b bitcnt(7)+1
	dc.b bitcnt(bitcnt(7))
	dc.b "\{bitcnt(7)}"
	dc.b bitcnt()
	dc.b bitcnt( )
	dc.b bitcnt(())
Later equ 7
	dc.b $EE
	end
