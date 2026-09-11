	cpu 68000
	padding off
	org 0
	charset $41,$11
	charset $27,$55
	charset $5C,$66
	charset $07,$77
	charset $0A,$AA
	charset $22,$BB
m	macro a
	dc.b a
	endm
	m "\x41"
	dc.b $EE
	end
