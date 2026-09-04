	cpu 68000
	org $1000
U struct
u:	ds.b 1
v:	ds.b 1
	endstruct
j:	U
	dc.w j_u-j,j_v-j,U_len
	dc.w j
