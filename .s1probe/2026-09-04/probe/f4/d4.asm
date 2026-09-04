	cpu 68000
	padding off
; no defaults at all
mp:	macro n1,n2,n3
	dc.b "s[ALLARGS]"
	shift
	dc.b "1[ALLARGS]"
	shift
	dc.b "2[ALLARGS]"
	endm
	mp aa
; default on LAST param only
ml:	macro n1,n2,n3=LL
	dc.b "s[ALLARGS]"
	shift
	dc.b "1[ALLARGS]"
	shift
	dc.b "2[ALLARGS]"
	endm
	ml aa
; default on FIRST param only, supplied nothing
mf:	macro n1=FF,n2,n3
	dc.b "s[ALLARGS]"
	shift
	dc.b "1[ALLARGS]"
	endm
	mf
	end
