	cpu 68000
	org 0
Start:
	dc.b 1,2,3
Back:
	rept Back-Start
	dc.b 0
	endr
	dc.b $FF
	end
