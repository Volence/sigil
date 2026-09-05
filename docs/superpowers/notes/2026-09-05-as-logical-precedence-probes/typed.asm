	cpu	68000
	padding	off
	org	0
; typed evaluator surface: the same three shapes inside int(), which routes
; through AS's typed (int XOR float) expression evaluator rather than the
; integer folder.
	dc.b	int(1&&2=2)
	dc.b	int(1+1<<3)
	dc.b	int(3!1|2)
	dc.b	int(2=2&&1)
	dc.b	int(1||2=2)
; and with a float operand present, which forces the typed domain
	dc.b	int(1.0+1<<3)
	dc.b	int(6.0/2*3)
