	cpu 68000
	padding off
q1:	macro p1,p2,p3=CC,p4
	dc.b "s[ALLARGS]<p1|p2|p3|p4>"
	shift
	dc.b "1[ALLARGS]<p1|p2|p3|p4>"
	shift
	dc.b "2[ALLARGS]<p1|p2|p3|p4>"
	endm
	q1 xx
q2:	macro p1,p2=BB,p3,p4=DD
	dc.b "s[ALLARGS]<p1|p2|p3|p4>"
	shift
	dc.b "1[ALLARGS]<p1|p2|p3|p4>"
	endm
	q2 xx
	end
