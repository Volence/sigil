	cpu 68000
	padding off
	org 0
V := W
	if V = 0
	fatal "first pass only, no MOMPASS involved"
	endif
	dc.b $11
W:	dc.b $EE
	end
