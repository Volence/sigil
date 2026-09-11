	cpu 68000
	padding off
	org 0
	asl	d1,d0
	asr	d2,d1
	lsl	d3,d2
	lsr	d4,d3
	rol	d5,d4
	ror	d6,d5
	roxl	d7,d6
	roxr	d0,d7
	dc.b $EE
	end
