	cpu 68000
	padding off
	org 0
s	:= substr("\x41\x42\x43",1,0)
	dc.b s
	dc.b $EE
	end
