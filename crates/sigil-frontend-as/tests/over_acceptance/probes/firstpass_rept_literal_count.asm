	cpu 68000
	org 0
Start:
	rept 3
	dc.b 0
	endr
End:
N	equ End-Start
	dc.b N
	end
