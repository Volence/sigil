	cpu	68000
	org	0
m	macro	px,py,pz
	dc.b	px,py,pz
	endm
	m	1,2<=3,4
	m	1,(2=3),4
	m	1,2=3,4
	m	1,py=,4
	m	1,=5,4
	end
