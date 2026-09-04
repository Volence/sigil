	cpu 68000
	padding off
	org 0
	enumconf 3
	enum a=5,b
	enum c,d
	nextenum e,f
	dc.b a,b,c,d,e,f
