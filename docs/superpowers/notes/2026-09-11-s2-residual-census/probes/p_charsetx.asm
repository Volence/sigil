	cpu 68000
	padding off
	org 0
	charset 'A',"\x10\x11\x12"
	dc.b "ABC"
	charset
	dc.b "ABC"
	end
