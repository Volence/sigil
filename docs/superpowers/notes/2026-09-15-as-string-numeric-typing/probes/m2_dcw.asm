	cpu 68000
	padding off
	org 0
	dc.w "a"
	dc.w "ab"
	dc.w "abc"
	dc.w "abcd"
	dc.w "a"+1
	dc.w "ab"+1
	dc.w "abc"+1
	dc.w 1+"ab"
	dc.w "a"+"b"
	dc.w "ab"+"cd"
	end
