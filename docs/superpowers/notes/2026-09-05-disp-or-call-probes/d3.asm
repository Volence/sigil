	cpu 68000
dsp	=	$2A
dsp	function p,(p*7)+$100
	padding off
	org $1000
	move.w #$1234,1+dsp(a1,zz)
