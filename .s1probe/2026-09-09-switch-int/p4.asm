	cpu 68000
	switch "btn"
		case "aaa"
			dc.b $11
		case "btn"
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
