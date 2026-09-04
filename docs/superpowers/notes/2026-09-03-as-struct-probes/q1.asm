	cpu 68000
	org $1000
S struct DOTS
a:	ds.b 1
b:	ds.w 1
c:	ds.l 1
	ds.b 3
d:	ds.b 1
	endstruct
	dc.w S.a, S.b, S.c, S.d
	dc.w S.len
	dc.w *
