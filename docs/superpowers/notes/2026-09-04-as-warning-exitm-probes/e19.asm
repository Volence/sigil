	cpu 68000
	padding off
	org 0
	rept 2
	dc.b $C0
	if 1
	exitm
	endif
	endr
LD:	dc.b $FF
	dc.l LD
