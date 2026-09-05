	cpu 68000
dsp	=	$2A
dsp	function p,(p*7)+$100
k	=	3
	padding off
	org $1000
	nop
	nop
	move.w	#$1234,dsp(k)+2(a1)
	; line 10 keeps line 11 where the note printed it
	move.w	#$1234,1+dsp(a1,d2.w)
