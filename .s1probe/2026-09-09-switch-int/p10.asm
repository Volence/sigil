	cpu 68000
V = 5
	switch V
		case 1
			dc.b $11
		elsecase
			dc.b $EE
		case 5
			dc.b $55
	endcase
	end
