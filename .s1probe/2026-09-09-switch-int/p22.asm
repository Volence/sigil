	cpu 68000
	switch 'A'
		case 65
			dc.b $11
		case "A"
			dc.b $22
		elsecase
			dc.b $EE
	endcase
V = 2
	switch V
		case "2"
			dc.b $33
		elsecase
			dc.b $44
	endcase
W = 65
	switch W+0
		case 'A'
			dc.b $55
		elsecase
			dc.b $66
	endcase
	end
