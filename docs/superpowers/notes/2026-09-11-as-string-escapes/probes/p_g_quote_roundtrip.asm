	cpu 68000
	padding off
	org 0
mq	macro pa
	dc.b pa
	dc.b strlen(pa)
	endm
	mq "a\\b\"c"
	dc.b $EE
	end
