	cpu 68000
	padding off
	org 0
	asl.b	d1,d0
	asl.l	#3,d2
	lsr.b	d0
	roxr.l	d7
	dc.b $EE
	end
