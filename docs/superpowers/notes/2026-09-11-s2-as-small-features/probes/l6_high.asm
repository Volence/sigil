	cpu 68000
	padding off
	org 0
	dc.l lastbit($80000000)
	dc.l lastbit($7FFFFFFF)
	dc.b $EE
	end
