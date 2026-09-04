	cpu 68000
	padding off
	rept 1+(abs(3-9)/abs(1))
	dc.b $AA
	endr
	dc.b [4]$FF
	move.w	#"SW",d0
zqm:	macro loc,port=(4).l
	move.l	#loc,port
	endm
	zqm	$1234
	zqm	$1234,d0
	switch 2
		case 1
	dc.b 1
		case 2
	dc.b 2
	endcase
	charset ' ', $FF
	dc.b " "
	charset
	end
