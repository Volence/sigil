	cpu 68000
	org $1000
f	function p,$3C7
	move.w	#f(5),d0
	end
