	cpu 68000
	padding off
	org 0
S1 equ "a"
S2 equ "ab"
S4 equ "abcd"
S5 equ "abcde"
	dc.b S1,$EE
	dc.b S2,$EE
	dc.b S5,$EE
	dc.w S2
	dc.l S2
	move.w #S1,d0
	move.w #S2,d0
	move.l #S4,d0
	move.w S2,d0
	dc.b S2+1,$EE
	dc.w S2+1
	move.w #S2+1,d0
	dc.b S1+S2,$EE
	dc.w S2-1
	end
