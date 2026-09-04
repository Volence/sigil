	cpu 68000
	padding off
	org $1000
; 1a: irpc over a literal string
	irpc c,"ABC"
	dc.b "<c>"
	endm
; 1b: irp over a comma list
	irp v,11,22,33
	dc.b v
	endm
; 1c: irpc terminator endr?
	irpc c,"XY"
	dc.b "[c]"
	endr
	end
