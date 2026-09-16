	cpu 68000
	padding off
	org 0
	dc.l "a"
	dc.l "ab"
	dc.l "abc"
	dc.l "abcd"
	dc.l "a"+1
	dc.l "ab"+1
	dc.l "abcd"+1
	dc.l "a"+"b"
	dc.l "ab"+"cd"
	end
