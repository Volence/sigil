	cpu 68000
	padding off
	org 0
	enumconf 4
	enum a=0,b
	enumconf 1
	nextenum c,d
	dc.b a,b,c,d
