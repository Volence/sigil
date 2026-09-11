	cpu 68000
	padding off
	org 0
zV STRUCT DOTS
	A:		ds.b 1
	2:	ds.b 1
zV ENDSTRUCT
	dc.b zV.len
	dc.b $EE
	end
