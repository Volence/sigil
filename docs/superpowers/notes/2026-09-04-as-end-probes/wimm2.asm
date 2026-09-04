	cpu 68000
	padding off
	phase 0
RAM_Start	= $FFFF0000
CrossResetRAM	= $FFFF8000
CrossResetRAM_End	= $FFFFFC00
	move.w	#((CrossResetRAM_End-CrossResetRAM)/4)-1,d6
	move.w	#((CrossResetRAM-RAM_Start)/4)-1,d6
	move.w	#(RAM_Start>>16)&$FFFF,d0
	lea	(CrossResetRAM).w,a6
	dc.l	RAM_Start&$FFFFFF
	end
