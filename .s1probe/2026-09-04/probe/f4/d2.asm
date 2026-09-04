	cpu 68000
	padding off
; ARGCOUNT / ALLARGS with defaults
ac:	macro p1,p2=DEF2,p3
	dc.b ARGCOUNT
	dc.b "<p1|p2|p3>"
	dc.b "[ALLARGS]"
	endm
	ac
	ac 11
	ac 11,22
	ac 11,22,33
	ac 11,,33
	ac p3=99
	end
