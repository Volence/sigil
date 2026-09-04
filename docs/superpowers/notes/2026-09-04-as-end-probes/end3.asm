	cpu 68000
	org 0
	dc.b $11
m	macro
	dc.b $55
	end
	dc.b $66
	endm
	m
	dc.b $22
	rept 3
	dc.b $77
	endm
	dc.b $33
	end	$1234
	dc.b $44
