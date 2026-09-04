	cpu 68000
	padding off
ml:	macro n1,n2,n3=LL
	dc.b "s[ALLARGS]<n1|n2|n3>"
	shift
	dc.b "1[ALLARGS]<n1|n2|n3>"
	shift
	dc.b "2[ALLARGS]<n1|n2|n3>"
	endm
	ml aa
mm:	macro n1,n2=DD,n3=EE
	dc.b "s[ALLARGS]<n1|n2|n3>"
	shift
	dc.b "1[ALLARGS]<n1|n2|n3>"
	shift
	dc.b "2[ALLARGS]<n1|n2|n3>"
	endm
	mm aa
	end
