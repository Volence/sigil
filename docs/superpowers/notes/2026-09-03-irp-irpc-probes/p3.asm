	cpu 68000
	padding off
	org $1000
one	macro pp
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	endm
three	macro q1,q2,q3
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	endm
	dc.w $A001
	one 11,22,33
	dc.w $A002
	one 11
	dc.w $A003
	one
	dc.w $A004
	three 11,22,33
	dc.w $A005
	three 11
	dc.w $A006
	three 11,22,33,44,55
	dc.w $A007
	end
