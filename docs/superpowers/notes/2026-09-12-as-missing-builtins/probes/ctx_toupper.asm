	cpu 68000
	padding off
	org 0
	dc.b toupper(FwdL)
	ds.b $3F
FwdL:
	dc.b toupper(97)
	dc.b TOUPPER(97)
	dc.b ToUpper(97)
	dc.b toupper( 97 )
	dc.b toupper((97))
	dc.b toupper(Later)
X equ toupper(97)
	dc.b X
Y set toupper(97)
	dc.b Y
	if toupper(97)=toupper(97)+1
	dc.b 2
	else
	dc.b 1
	endif
	move.l #toupper(97),d0
	move.w #toupper(97)<<2,d1
	dc.b toupper(97)+1
	dc.b toupper(toupper(97))
	dc.b "\{toupper(97)}"
	dc.b toupper()
	dc.b toupper( )
	dc.b toupper(())
Later equ 97
	dc.b $EE
	end
