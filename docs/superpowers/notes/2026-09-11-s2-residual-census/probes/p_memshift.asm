	cpu 68000
	padding off
	org 0
	asl	$1A(a0)
	asr	$18(a0)
	asr	$1A(a1)
	asl.w	$1A(a0)
	lsr	$10(a2)
	rol	$10(a2)
	end
