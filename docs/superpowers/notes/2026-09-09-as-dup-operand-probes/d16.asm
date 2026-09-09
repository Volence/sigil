; d16 - is the bracket group gated on the CPU or on the directive spelling?
; d14 shows `db [3]v` refused under `cpu z80`. This asks whether `db`/`dw`
; take it under `cpu 68000`.
	cpu	68000
	padding	off
	org	$1000
	db	[3]$FF
	dw	[2]$1234
	end
