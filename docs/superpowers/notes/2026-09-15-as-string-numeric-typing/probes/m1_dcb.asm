	cpu 68000
	padding off
	org 0
	dc.b "a",$EE
	dc.b "ab",$EE
	dc.b "abc",$EE
	dc.b "abcd",$EE
	dc.b "abcde",$EE
	dc.b "a"+1,$EE
	dc.b "ab"+1,$EE
	dc.b "abc"+1,$EE
	dc.b "abcd"+1,$EE
	dc.b 1+"ab",$EE
	dc.b "a"+"b",$EE
	dc.b "ab"+"cd",$EE
	end
