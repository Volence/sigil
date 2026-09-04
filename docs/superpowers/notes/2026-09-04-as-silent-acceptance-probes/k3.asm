	cpu	68000
	org	0
m	macro	px,py,pz
	dc.b	px,py,pz
	endm
zz	equ	7
	m	1,zz=2,3
	m	1,2,px=9
	m	px=1,px=2,pz=3
	end
