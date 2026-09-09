; d14 - `[count]value` on the Z80 surface. Z80 hex is `NNNNh`, not `$NNNN`:
; under `cpu z80` AS reads `$` as the location counter.
	cpu	z80
	org	1000h
	db	[3]0FFh
	dw	[2]1234h
	end
