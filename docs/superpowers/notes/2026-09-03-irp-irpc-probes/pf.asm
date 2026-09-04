	cpu 68000
	padding off
	org $1000
sh	macro aa
	dc.b "1<aa>"
	shift
	dc.b "2<aa>"
	shift
	dc.b "3<aa>"
	shift
	dc.b "4<aa>"
	dc.b $EE
	dc.b "5<aa>"
	endm
	sh p1,p2,p3
	dc.w $A701
	end
