	cpu 68000
	padding off
im:	macro {INTLABEL},pp,qq=QQ
	dc.b "<__LABEL__|pp|qq>"
	endm
Lb:	im 11
	end
