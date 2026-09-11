	cpu 68000
	padding off
	org 0
	charset $41,$11
	charset $27,$55
	charset $5C,$66
	charset $07,$77
	charset $0A,$AA
	charset $22,$BB
s	:= "\x41"
	dc.b s
	dc.b $EE
	end
