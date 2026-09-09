	cpu 68000
V = 65
	switch V
		case 'B'
			dc.b $11
		case 'A'
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
