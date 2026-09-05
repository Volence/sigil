	cpu 68000
dsp	=	$2A
dsp	function p,(p*7)+$100
k	=	3
	padding off
	org $1000
	move.w	#$1234,dsp(a1)
	; line 8 keeps line 9 where the note printed it
	move.w	#$1234,1+dsp(a1)
	; line 10 keeps line 11 where the note printed it
	move.w	#dsp(k),d0
