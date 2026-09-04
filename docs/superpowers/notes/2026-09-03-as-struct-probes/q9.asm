	cpu 68000
	org $1000
W struct DOTS
w:	ds.w 1
	endstruct
	org $2001
i1:	W
a1:
	org $3001
i2:	ds.w 1
a2:
	dc.w i1,a1,i2,a2
	dc.w W.len
