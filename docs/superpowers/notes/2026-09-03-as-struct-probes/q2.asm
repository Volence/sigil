	cpu 68000
	padding off
	org $1000
S struct DOTS
a:	ds.b 1
b:	ds.w 1
c:	ds.l 1
	endstruct
T struct DOTS
a:	ds.b 1
b:	ds.b 2
	endstruct
	dc.w S.a,S.b,S.c,S.len
	dc.w T.a,T.b,T.len
