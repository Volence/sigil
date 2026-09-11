	cpu 68000
	padding off
	org 0
	asl.w	#1,d0
	asr.w	#2,d1
	lsl.w	#3,d2
	lsr.w	#4,d3
	rol.w	#5,d4
	ror.w	#6,d5
	roxl.w	#7,d6
	roxr.w	#8,d7
	dc.b $EE
	end
