	cpu 68000
V = 4
W = 2
	switch V*1
		case W-1
			dc.b $11
		case W+2
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
