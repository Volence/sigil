	cpu 68000
	padding off
	org 0
	dc.l INT(acosh())
	dc.b $EE
	end
