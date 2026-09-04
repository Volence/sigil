	cpu	68000
	org	0
m	macro	px,py,pz
	dc.b	px,py,pz
	endm
	m	1,2,3
	m	px=1,py=2,pz=3
	m	py=$22,1,3
	m	1,py=$22,3
	end
