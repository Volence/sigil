	cpu 68000
	padding off
	org 0
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying	ds.b 1
	B:		ds.b 1
zV ENDSTRUCT
	dc.b zV.B
	dc.b zV.len
	dc.b $EE
	end
