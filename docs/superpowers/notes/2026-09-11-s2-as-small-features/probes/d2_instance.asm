	cpu 68000
	padding off
	org 0
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying:	ds.b 1
zV ENDSTRUCT
	dc.b $11
Inst:	zV
	dc.l Inst.1upPlaying
	dc.b $EE
	end
