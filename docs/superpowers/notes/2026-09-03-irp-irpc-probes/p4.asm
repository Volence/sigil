	cpu 68000
	padding off
	org $1000
m2	macro q1,q2
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
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
m4	macro q1,q2,q3,q4
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
	dc.w ARGCOUNT
	shift
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
	dc.w $B200
	m2 
	dc.w $B201
	m2 10
	dc.w $B202
	m2 10,11
	dc.w $B204
	m2 10,11,12,13
	dc.w $B206
	m2 10,11,12,13,14,15
	dc.w $B400
	m4 
	dc.w $B401
	m4 10
	dc.w $B402
	m4 10,11
	dc.w $B404
	m4 10,11,12,13
	dc.w $B406
	m4 10,11,12,13,14,15
	end
