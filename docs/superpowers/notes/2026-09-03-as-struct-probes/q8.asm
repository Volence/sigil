	cpu 68000
	org $1000
A struct
a:	ds.b 1
b:	ds.b 1
	endstruct
B struct DOTS
a:	ds.b 1
	ends
C struct DOTS
a:	ds.b 1
	endstruct C
	dc.w A_a,A_b,A_len
	dc.w B.a,B.len
	dc.w C.a,C.len
	A
noLabel:
	dc.w noLabel
