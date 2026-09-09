	cpu 68000
V = 2
	switch V
		case 1
			dc.b $11
		case 2
			dc.b $22
		case 3
			dc.b $33
		elsecase
			dc.b $EE
	endcase
	end
