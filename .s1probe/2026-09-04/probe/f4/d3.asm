	cpu 68000
	padding off
; shift interaction with defaults
sh:	macro n1,n2=DD,n3
	dc.b "s[ALLARGS]"
	shift
	dc.b "1[ALLARGS]<n1|n2|n3>"
	shift
	dc.b "2[ALLARGS]<n1|n2|n3>"
	endm
	sh aa
	sh aa,bb,cc
; default boundary: parenthesised comma, spaces, string default
b1:	macro q=(1,2),r=ZZ
	dc.b "<q|r>"
	endm
	b1
b2:	macro s = 5 , t = 6
	dc.b "<s|t>"
	endm
	b2
b3:	macro u="hi",v=3+4
	dc.b "<u|v>"
	dc.b u
	dc.b v
	endm
	b3
	end
