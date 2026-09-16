	cpu 68000
	padding off
	org 0
	dc.b "a"+1,$EE
	dc.b "ab"+1,$EE
	dc.b "abc"+1,$EE
	dc.b "abcd"+1,$EE
	dc.b 1+"ab",$EE
	dc.b "aa"+255,$EE
	dc.b "ab"+256,$EE
	move.w #"ab"+1,d0
	move.l #"abcd"+1,d0
	move.w "ab"+1,d0
	end
