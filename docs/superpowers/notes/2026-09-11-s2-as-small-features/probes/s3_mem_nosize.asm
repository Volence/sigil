	cpu 68000
	padding off
	org 0
	asl	$1A(a0)
	asr	(a1)
	lsl	(a2)+
	lsr	-(a3)
	rol	$10(a4,d0.w)
	ror	$1234.w
	roxl	$12345678.l
	roxr	$18(a0)
	dc.b $EE
	end
