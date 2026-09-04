	cpu 68000
	org $0
H struct dots
a:	ds.l	1
b:	ds.l	1
c:	ds.b	1
H endstruct
	dc.w H.a,H.b,H.c,H.len
