	cpu 68000
	org $1000
f	function p,(p*7)+$100
	move.w	#f(a1),d0
	end
