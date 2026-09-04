	cpu 68000
	padding off
vdp_control_port equ 4
zqm:	macro loc,port=(vdp_control_port).l
	move.l	#loc,port
	endm
	zqm	$1234
	zqm	$1234,d0
	zqm	loc=$5678
	zqm	$1234,
	end
