	cpu 68000
	padding off
	org 0
s	:= "\x41\66"
	dc.b s
	dc.b strlen(s)
	dc.b $EE
	end
