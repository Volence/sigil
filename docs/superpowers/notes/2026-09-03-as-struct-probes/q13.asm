	cpu z80
	org $1000
Z struct DOTS
a:	ds.b 1
b:	ds.w 1
c:	ds.b 1
d:	ds.l 1
	endstruct
	db Z.a,Z.b,Z.c,Z.d,Z.len
i:	Z
	db i.b-i, i.d-i
