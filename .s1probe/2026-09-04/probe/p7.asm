	cpu 68000
	padding off
Revision = 1
	dc.l	LblOnIf
	dc.l	LblOnRept
	dc.l	LblOnInclude
LblOnIf:    if Revision=0
	dc.b	$00
	    else
	dc.b	$01
	    endif
LblOnRept:  rept 2
	dc.b	$02
	endr
LblOnInclude:	dc.b	$03
	end
