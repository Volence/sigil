	cpu 68000
	padding off
	org $1000
t3	macro pp,qq,rr
	dc.b "0<pp|qq|rr>"
	shift
	dc.b "1<pp|qq|rr>"
	shift
	dc.b "2<pp|qq|rr>"
	shift
	dc.b "3<pp|qq|rr>"
	endm
	t3 a1,a2,a3
	dc.b $EE
	t3 a1,a2,a3,a4
	dc.b $EE
	t3 a1
	dc.w $A801
	end
