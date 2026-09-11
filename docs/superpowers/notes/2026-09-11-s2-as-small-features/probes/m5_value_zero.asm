	cpu 68000
	padding off
	org 0
val	macro n
	dc.b n
	endm
	val 007
	dc.b $EE
	end
