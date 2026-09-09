	cpu 68000
V = 9
	switch V
		case 1
			dc.b $11
		case 2
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
