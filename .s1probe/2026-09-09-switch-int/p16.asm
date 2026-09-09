	cpu 68000
	switch "bee"
		case "aaa","zzz"
			dc.b $11
		case 3,"bee"
			dc.b $22
		elsecase
			dc.b $EE
	endcase
	end
