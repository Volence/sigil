	cpu 68000
	padding off
	org 0
	dc.b tolower(FwdL)
	ds.b $3F
FwdL:
	dc.b tolower(65)
	dc.b TOLOWER(65)
	dc.b ToLower(65)
	dc.b tolower( 65 )
	dc.b tolower((65))
	dc.b tolower(Later)
X equ tolower(65)
	dc.b X
Y set tolower(65)
	dc.b Y
	if tolower(65)=tolower(65)+1
	dc.b 2
	else
	dc.b 1
	endif
	move.l #tolower(65),d0
	move.w #tolower(65)<<2,d1
	dc.b tolower(65)+1
	dc.b tolower(tolower(65))
	dc.b "\{tolower(65)}"
	dc.b tolower()
	dc.b tolower( )
	dc.b tolower(())
Later equ 65
	dc.b $EE
	end
