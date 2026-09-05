	cpu 68000
	org $1000
f	function p,$3C7
	move.w	#f(a1),d0
	end
