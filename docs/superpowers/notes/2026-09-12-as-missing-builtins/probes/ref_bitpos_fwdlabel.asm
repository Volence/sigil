	cpu 68000
	padding off
	org 0
	dc.b bitpos(FwdL)
	ds.b $3F
FwdL:
	dc.b $EE
	end
