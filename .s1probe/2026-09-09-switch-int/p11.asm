	cpu 68000
	switch Undef
		case 1
			dc.b $11
		case 2
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
