	cpu 68000
	padding off
	org 0
	dc.w "abcd"+65536
	dc.b "\xff\xff\xff\xff"+1,$EE
	dc.b "\xff\xff\xff"+1,$EE
	dc.b "abcd"+"efgh"+1,$EE
	end
