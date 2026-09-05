	cpu 68000
konst	=	$2A
konst	function p,$3C7
	org $1000
	move.w	#konst(5),d0
	move.w	#konst(a1),d0
