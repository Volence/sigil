	cpu 68000
	padding off
	org 0
zV STRUCT
	A:		ds.b 1
	1upPlaying:	ds.b 1
	B:		ds.b 1
zV ENDSTRUCT
	dc.b zV_1upPlaying
	dc.b zV_B
	dc.b zV_len
	dc.b $EE
	end
