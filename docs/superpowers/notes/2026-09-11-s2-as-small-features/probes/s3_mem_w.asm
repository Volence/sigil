	cpu 68000
	padding off
	org 0
	asl.w	$1A(a0)
	asr.w	(a1)
	lsl.w	(a2)+
	lsr.w	-(a3)
	rol.w	$10(a4,d0.w)
	ror.w	$1234.w
	roxl.w	$12345678.l
	roxr.w	$18(a0)
	dc.b $EE
	end
