	cpu 68000
	org $1000
T struct DOTS
p:	ds.b 1
r:	ds.w 1
	endstruct
S struct DOTS
h:	ds.b 1
n:	T
z:	ds.b 1
	endstruct
U struct
u:	ds.b 1
	endstruct
	dc.w T.p,T.r,T.len
	dc.w S.h,S.n,S.n.p,S.n.r,S.z,S.len
i:	S
	dc.w i.n.p-i,i.n.r-i,i.z-i
j:	U
	dc.w j.u-j,U_len
