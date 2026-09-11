	cpu 68000
	padding off
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying:	ds.b 1
	B:		ds.b 1
zV ENDSTRUCT
	org 0
	dc.b zV.1upPlaying
	dc.b zV.B
	dc.b zV.len
	end
