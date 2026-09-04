	cpu 68000
	padding off
	org $1000
removeJmpTos = 0
	dc.b $01,~~removeJmpTos
inner	macro
	dc.b $02,ARGCOUNT
	irp op,ALLARGS
	dc.b "<op>"
	endm
	endm
outer	macro UseNop
	dc.b $03,ARGCOUNT
	shift
	dc.b $04,ARGCOUNT
	inner ALLARGS
	endm
top	macro
	dc.b $05,ARGCOUNT
	outer TRUE,ALLARGS
	endm
	top zz1,zz2
	dc.b $FF
	top
	end
