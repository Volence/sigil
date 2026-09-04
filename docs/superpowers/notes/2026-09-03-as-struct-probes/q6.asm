	cpu 68000
	org $1000
T struct DOTS
p:	ds.b 1
q:	ds.b 1
	endstruct
S struct DOTS
h:	ds.b 1
	ds.b 1
n:	T
m:	T
e:
	endstruct
	dc.w T.p,T.q,T.len
	dc.w S.h,S.n,S.m,S.e,S.len
	dc.w S.n.p,S.n.q,S.m.p,S.m.q
inst:	S
	dc.w inst,inst.h,inst.n,inst.n.p,inst.n.q,inst.m.q,inst.e
