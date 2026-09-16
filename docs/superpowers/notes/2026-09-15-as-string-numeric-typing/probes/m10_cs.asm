	cpu 68000
	padding off
	org 0
	charset "a",$11
	dc.b "ab",$EE
	dc.b "ab"+1,$EE
	dc.b "a"+"b",$EE
	move.w #"ab",d0
	move.w #"ab"+1,d0
	end
