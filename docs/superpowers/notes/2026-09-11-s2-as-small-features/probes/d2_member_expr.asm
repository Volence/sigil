	cpu 68000
	padding off
	org 0
zV STRUCT DOTS
	A:		ds.b 2
	1upPlaying:	ds.b 1
zV ENDSTRUCT
	dc.b zV.1upPlaying+1
	dc.b 2*zV.1upPlaying
	dc.b $EE
	end
