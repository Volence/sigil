	cpu 68000
	org $1000
S STRUCT DOTS
	a:	ds.b 1
	b:	ds.b 1
S ENDSTRUCT
V struct dots
	ds.b 3
V endstruct
	dc.w S.a,S.b,S.len
	dc.w V.len
k:	S
	dc.w k.a-k,k.b-k
m:	V
n:
	dc.w n-m
