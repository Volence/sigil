	cpu 68000
	org $1000
W struct DOTS
w:	ds.w 1
x:	ds.b 1
	endstruct
	dc.w i1,after,i1.w-i1,i1.x-i1
	ds.b 1
i1:	W
after:
