	cpu 68000
	padding off
	org 0
	dc.b "ab"+"cd"+"ef",$EE
	dc.b "A"+"",$EE
	dc.b ""+"B",$EE
	dc.b ""+"",$EE
	dc.b ("a"+"b")+"c",$EE
T set "ab"+"cd"
	dc.b strlen(T),$EE
	dc.b 1+2,$EE
	dc.b "a"+1,$EE
	dc.b substr("hello",1,2)+"-"+lowstring("XY"),$EE
	end
