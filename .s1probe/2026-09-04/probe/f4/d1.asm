	cpu 68000
	padding off
vdp_control_port equ 4
zqm:	macro loc,port=(vdp_control_port).l
	move.l	#loc,port
	endm
; A: bare call, default applies
	zqm	$1234
; B: explicit second arg
	zqm	$1234,d0
; C: keyword form for the FIRST param, default for second
	zqm	loc=$5678
; D: trailing empty positional -- default or empty?
	zqm	$1234,
	end
