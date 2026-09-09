	cpu 68000
V = 1
	switch V
		case Undef,1
			dc.b $11
		case 2
			dc.b $22
	endcase
	end
