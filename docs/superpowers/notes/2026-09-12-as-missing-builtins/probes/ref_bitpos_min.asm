	cpu 68000
	padding off
	org 0
	dc.l bitpos(-$7FFFFFFFFFFFFFFF-1)
	dc.b $EE
	end
