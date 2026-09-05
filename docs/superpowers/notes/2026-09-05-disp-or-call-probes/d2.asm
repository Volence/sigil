	cpu 68000
konst	function p,$3C7
	padding off
	org $1000
	move.w #$1234,konst(a1)
