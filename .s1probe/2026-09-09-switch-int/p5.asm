	cpu 68000
V = 3
	switch V
		case 1,2
			dc.b $11
		case 3,4
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
