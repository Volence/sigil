	cpu 68000
	padding off
	org 0
	asl	#1,d0
	asr	#2,d1
	lsl	#3,d2
	lsr	#4,d3
	rol	#5,d4
	ror	#6,d5
	roxl	#7,d6
	roxr	#8,d7
	dc.b $EE
	end
