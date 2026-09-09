	cpu 68000
	dc.b 'A'+1
	dc.l 'INIT'
	if 65='A'
		dc.b $AA
	else
		dc.b $BB
	endif
V = 65
	if V='A'
		dc.b $CC
	else
		dc.b $DD
	endif
	end
