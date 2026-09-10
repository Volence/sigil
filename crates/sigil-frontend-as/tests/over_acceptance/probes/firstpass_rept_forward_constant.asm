	cpu 68000
	org 0
Start:
	rept N
	dc.b 0
	endr
End:
N	equ 4
	dc.b $FF
	end
