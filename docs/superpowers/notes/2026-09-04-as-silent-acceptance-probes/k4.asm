	cpu	68000
	org	0
m	macro	px,py,pz
	dc.b	px,py,pz
	endm
	m	1,2+3=5,4
	m	1,<py=2>,3
	m	1,"py=2",3
	m	1,px =9,3
	m	PX=1,2,3
	end
