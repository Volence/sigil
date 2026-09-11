	cpu 68000
	padding off
	org 0
	asl	(a0)
	asr	(a0)
	lsl	(a0)
	lsr	(a0)
	rol	(a0)
	ror	(a0)
	roxl	(a0)
	roxr	(a0)
	dc.b $EE
	end
