	cpu 68000
	enum Ea,Eb
	ifdef Eb
	dc.b $A1
	endif
	ifdef Ec
	dc.b $A2
	endif
	enum Ec
	dc.b 0
