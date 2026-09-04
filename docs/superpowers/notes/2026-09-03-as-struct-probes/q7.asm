	cpu 68000
	org $1000
S struct DOTS
a:	ds.b 1
b:	ds.w 1
c:	ds.b 1
d:	ds.l 1
	endstruct
	dc.w S.a,S.b,S.c,S.d,S.len
inst:	S
	dc.w inst.a-inst,inst.b-inst,inst.c-inst,inst.d-inst
