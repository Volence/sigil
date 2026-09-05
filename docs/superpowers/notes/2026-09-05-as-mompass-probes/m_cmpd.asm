	cpu 68000
	padding off
	org 0
Z = 3
N = 4
	if (Z<>N)&&(MOMPASS=1)
	dc.b $AA
	endif
	dc.b $11
	dc.w Later-*
Later:
	end
