	cpu 68000
	padding off
	org 0
	move.l #"\x41\x42\x43\x44",d0
	dc.b $EE
	end
