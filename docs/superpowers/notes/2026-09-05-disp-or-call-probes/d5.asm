	cpu 68000
konst	=	$2A
konst	function p,$3C7
	padding off
	org $1000
	move.w	#$1234,1+konst(a1)
