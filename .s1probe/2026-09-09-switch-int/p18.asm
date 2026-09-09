	cpu 68000
V = 1
	switch V
		case 1
			dc.b $11
		case Undef
			dc.b $22
		case
			dc.b $33
	endcase
	end
