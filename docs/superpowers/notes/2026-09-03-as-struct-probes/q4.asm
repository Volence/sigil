	cpu 68000
	org $1001
x:	dc.w $1234
	org $2001
y:	ds.w 1
	org $3001
z:	ds.b 1
	org $4000
	dc.l x,y,z
