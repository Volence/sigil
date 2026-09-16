	cpu 68000
	padding off
	org 0
	dc.b "\x00a"+1,$EE
	dc.b "\x00ab"+1,$EE
	dc.b "\xff\xff\xff"+1,$EE
	dc.b "a"+(0-97),$EE
	dc.b "a"+(-98),$EE
	dc.b "a"+(0-300),$EE
	dc.b "ab"+(0-30000),$EE
	dc.b "ab"+(0-1),$EE
	dc.b "a"+255,$EE
	dc.b "\xfe\xff\xff\xff"+$20000,$EE
	end
