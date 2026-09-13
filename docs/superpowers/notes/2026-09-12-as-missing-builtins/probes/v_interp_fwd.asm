	cpu 68000
	padding off
	org 0
	dc.b "\{Later}",$EE
	dc.b substr("\{Later}",0,0),$EE
	dc.b "-"+"\{Later}",$EE
Later equ 5
	end
