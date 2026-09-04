	cpu 68000
	padding off
	org 0
	enum a=$80,b,c=b,d,e
	dc.b a,b,c,d,e
