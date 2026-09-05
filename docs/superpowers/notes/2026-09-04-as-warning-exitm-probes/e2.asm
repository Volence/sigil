	cpu 68000
	padding off
	org 0
inner macro
	dc.b $A0
	exitm
	dc.b $A1
	endm
outer macro
	dc.b $B0
	inner
	dc.b $B1
	endm
	outer
	dc.b $FF
