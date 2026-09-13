	cpu 68000
	padding off
	org 0
	dc.b substr("$\{abs(-5)}",0,0),$EE
	dc.b lowstring("A\{abs(-5)}B"),$EE
	dc.b "-"+"$\{abs(-5)}",$EE
	dc.b "\{abs(-5)}"+"!",$EE
	end
