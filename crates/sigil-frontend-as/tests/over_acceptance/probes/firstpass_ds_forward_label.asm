	cpu 68000
	org 0
Start:
	ds.b N
End:
N	equ End-Start+1
	dc.b $FF
	end
