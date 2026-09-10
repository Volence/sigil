	cpu 68000
	org 0
Start:
	if End-Start>0
	dc.b 0,0,0
	endif
End:
	dc.b $FF
	end
