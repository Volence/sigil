	cpu 68000
	padding off
	org 0
	org $123
	dc.l 2<<lastbit(*-1)
	dc.b $EE
	end
