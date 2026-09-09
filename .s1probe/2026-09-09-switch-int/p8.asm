	cpu 68000
	switch "btn"
		case "aaa"
			dc.b $11
		case "zzz"
			dc.b $22
	endcase
	dc.b $FF
	end
