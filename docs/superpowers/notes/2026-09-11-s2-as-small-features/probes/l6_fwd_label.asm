	cpu 68000
	padding off
	org 0
	dc.b lastbit(Later)
	ds.b $40
Later:	dc.b 1
	dc.b $EE
	end
