	cpu 68000
	switch "2"
		case 2
			dc.b $11
		case "2"
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
