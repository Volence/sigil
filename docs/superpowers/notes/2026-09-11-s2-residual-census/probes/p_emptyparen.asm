	cpu 68000
	padding off
	org 0
FLAGSV = 0
	dc.b $10|()|$02
	dc.b ()
	dc.b (FLAGSV)|$20
	end
