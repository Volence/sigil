	cpu 68000
	padding off
	org 0
mq	macro pa,pb
	dc.b pa
	dc.b pb
	endm
	mq "\"x,y",$11
	dc.b $EE
	end
