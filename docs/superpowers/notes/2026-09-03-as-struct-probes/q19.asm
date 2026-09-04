	cpu 68000
	org $0
V struct dots
	a:	ds.b 1
	1upPlaying:	ds.b 1
	b:	ds.b 1
V endstruct
	dc.w V.a,V.b,V.len
