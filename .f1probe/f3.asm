	cpu 68000
	padding off
fx = 3.7
; --- float symbol in an integer context
	dc.l fx
; --- float in an immediate operand
	move.l #INT(3.7),d0
	move.l #3.7,d0
; --- float literal forms
	dc.l INT(1e3)
	dc.l INT(1E3)
	dc.l INT(1.5e2)
	dc.l INT(.5+3)
	dc.l INT(1.)
	dc.l INT(2.5e-1*8)
; --- integer-only operators on floats
	dc.l INT(7.5%2)
	dc.l INT(7.5&3)
	dc.l INT(7.5<<1)
	dc.l INT(7.5!3)
; --- float in if / rept / org contexts
	if 3.5>2
	dc.b $AA
	endif
	rept INT(2.9)
	dc.b $BB
	endm
; --- comparison chain used by min function
	dc.l 5!((5!9)&(-(5<9)))
	end
