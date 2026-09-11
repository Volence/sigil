	cpu 68000
	padding off
	org 0
	asl.w	d1,d0
	asr.w	d2,d1
	lsl.w	d3,d2
	lsr.w	d4,d3
	rol.w	d5,d4
	ror.w	d6,d5
	roxl.w	d7,d6
	roxr.w	d0,d7
	dc.b $EE
	end
