	cpu 68000
	padding off
	org $1000
ac	macro
	dc.w ARGCOUNT
	endm
	dc.w $A101
	ac
	dc.w $A102
	ac 
	dc.w $A103
	ac ,
	dc.w $A104
	ac 1,
	dc.w $A105
	ac ,1
	dc.w $A106
	ac 1,,3
	dc.w $A107
; ARGCOUNT via ALLARGS relay, the jmpTos shape
relay	macro UseNop
	shift
	ac ALLARGS
	endm
	dc.w $A201
	relay TRUE,
	dc.w $A202
	relay TRUE,zz1,zz2
	dc.w $A203
	end
