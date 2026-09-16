	cpu 68000
	padding off
	org 0
	dc.b "ab"+256,$EE
	dc.b "ab"+1,$EE
	dc.b "ab"-1,$EE
	dc.b "aa"+255,$EE
	dc.b "abcde"+1,$EE
	dc.b "abcdefgh"+1,$EE
	dc.b "ab"*2,$EE
	dc.b "ab"&$00FF,$EE
	dc.b "ab">>8,$EE
	dc.b -"ab",$EE
	end
