	cpu 68000
	padding off
	phase 0
m	macro	px,py,pz
	dc.b	px,py,pz
	endm
	m	1,2,px=9
