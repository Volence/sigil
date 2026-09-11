	cpu 68000
	padding off
	org 0
	move.l #2<<lastbit($FFFEB),d0
	dc.b $EE
	end
