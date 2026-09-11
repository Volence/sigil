	cpu 68000
	org 0
Lbl:
	dc.l (NoSuchPlc<<24)|Lbl
	jsr NoSuchTarget
	dc.w $8000,$8000,$8000,$8000
