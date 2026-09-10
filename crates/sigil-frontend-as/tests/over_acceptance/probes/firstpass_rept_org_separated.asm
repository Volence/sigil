	cpu 68000
	org 0
Start:
	rept N
	dc.b 0
	endr
	org $100
End:
N	equ End-Start
	dc.b $FF
	end
