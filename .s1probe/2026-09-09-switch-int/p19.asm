	cpu 68000
V = 1
	switch V
		case 1,Undef
			dc.b $11
		case 2
			dc.b $22
	endcase
	end
