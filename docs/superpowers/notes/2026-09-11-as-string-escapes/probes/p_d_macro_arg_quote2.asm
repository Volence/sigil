	cpu 68000
	padding off
	org 0
mq	macro pa
	dc.b pa
	endm
	mq "a\";b"
	dc.b $EE
	end
