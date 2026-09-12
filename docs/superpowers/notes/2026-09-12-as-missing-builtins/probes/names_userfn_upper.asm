	cpu 68000
	padding off
	org 0
SGN function x,x+100
BITCNT function x,x+100
FIRSTBIT function x,x+100
BITPOS function x,x+100
TOUPPER function x,x+100
TOLOWER function x,x+100
LASTBIT function x,x+100
ABS function x,x+100
INT function x,x+100
	dc.b sgn(8)
	dc.b bitcnt(8)
	dc.b firstbit(8)
	dc.b bitpos(8)
	dc.b toupper(8)
	dc.b tolower(8)
	dc.b lastbit(8)
	dc.b abs(8)
	dc.b int(8)
	dc.b $EE
	end
