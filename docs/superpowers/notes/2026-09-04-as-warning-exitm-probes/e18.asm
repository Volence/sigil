	cpu 68000
	padding off
	org 0
	rept 2
	dc.b $C0
	if 1
	exitm
LC:	endif
	endr
	dc.b $FF
	dc.l LC
