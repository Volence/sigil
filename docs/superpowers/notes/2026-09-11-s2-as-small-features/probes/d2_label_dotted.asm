	cpu 68000
	padding off
	org 0
	dc.b $11
Foo.1up:	dc.b $22
	dc.b Foo.1up
	dc.b $EE
	end
