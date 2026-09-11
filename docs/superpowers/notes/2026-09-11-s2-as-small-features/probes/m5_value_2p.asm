	cpu 68000
	padding off
	org 0
val	macro n
	dc.b n
	endm
	val 2p
	dc.b $EE
	end
