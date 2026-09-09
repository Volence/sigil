	cpu 68000
V = 2
	switch V
		case 1
			dc.b $11
		case 2
			dc.b $22
		case 2
			dc.b $BB
	endcase
	end
