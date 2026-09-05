	cpu 68000
	padding off
	org 0
FLAG := 0
V := W
	if MOMPASS=2
FLAG := $AA
	endif
	dc.b FLAG
	dc.b $11
	dc.b V
W:	dc.b $EE
	end
