	cpu 68000
	ifdef MOMPASS
	dc.b $A1
	endif
	ifdef TRUE
	dc.b $A2
	endif
	ifdef MOMCPU
	dc.b $A3
	endif
	dc.b 0
