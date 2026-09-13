	cpu 68000
	padding off
	org 0
	dc.b "\{strlen("ab")}",$EE
	dc.b "\{strlen("ab"+"cd")}",$EE
	end
