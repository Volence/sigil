	cpu 68000
	padding off
	org 0
	enumconf $C
	enum a=$88,b,c
	dc.b a,b,c
