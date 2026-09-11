	cpu 68000
	padding off
	org 0
	dc.b substr("\x41\x42\x43",1,1)
	dc.b $EE
	end
