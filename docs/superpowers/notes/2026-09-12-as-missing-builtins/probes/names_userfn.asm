	cpu 68000
	padding off
	org 0
sgn function x,x+100
bitcnt function x,x+100
firstbit function x,x+100
bitpos function x,x+100
toupper function x,x+100
tolower function x,x+100
lastbit function x,x+100
abs function x,x+100
int function x,x+100
	dc.b sgn(1)
	dc.b bitcnt(1)
	dc.b firstbit(1)
	dc.b bitpos(1)
	dc.b toupper(1)
	dc.b tolower(1)
	dc.b lastbit(1)
	dc.b abs(1)
	dc.b int(1)
	dc.b $EE
	end
