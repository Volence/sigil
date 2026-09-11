	cpu 68000
	padding off
	org 0
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying:	ds.b 1
zV ENDSTRUCT
	dc.b zV.1foo
	dc.b $EE
	end
