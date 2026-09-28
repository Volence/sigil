	cpu 68000
	switch N
	case 1
	dc.b 1
	case 2
	dc.b 2
	elsecase
	dc.b $FF
	endcase
N equ 2
