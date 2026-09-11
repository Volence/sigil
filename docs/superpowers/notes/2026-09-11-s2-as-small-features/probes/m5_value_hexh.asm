	cpu 68000
	padding off
	org 0
val	macro n
	dc.b n
	endm
	val 12h
	dc.b $EE
	end
