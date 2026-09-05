	cpu 68000
	org $1000
dsp	function p,(p*7)+$100
	move.w	#$1234,1+dsp(d3).w
	end
