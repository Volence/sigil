	cpu 68000
V = 2
	switch V
		case 1
Sel = 1
		case 2
Sel = 2
		elsecase
Sel = 3
	endcase
	message "ARM TAKEN: Sel=\{Sel}"
	end
