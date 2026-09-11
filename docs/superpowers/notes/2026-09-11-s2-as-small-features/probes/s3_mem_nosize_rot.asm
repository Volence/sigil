	cpu 68000
	padding off
	org 0
	asl	(a1)
	asr	(a2)+
	lsl	-(a3)
	lsr	$10(a4,d0.w)
	rol	$1234.w
	ror	$12345678.l
	roxl	$18(a0)
	roxr	$1A(a0)
	dc.b $EE
	end
