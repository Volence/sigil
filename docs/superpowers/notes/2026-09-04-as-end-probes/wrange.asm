	cpu 68000
	padding off
	phase 0
	move.w	#65535,d0
	move.w	#-32768,d0
	move.w	#-32769,d0
	move.w	#65536,d0
	move.w	#-65536,d0
	move.w	#$FFFFF700,d0
	end
