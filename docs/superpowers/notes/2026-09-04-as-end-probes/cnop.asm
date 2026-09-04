; The s2disasm cnop/align macro pair, verbatim in shape from
; s2.macrosetup.asm:38-51 (the `notZ80(MOMCPU)` arm), which is the site the
; booked AS-CNOP-ORG-CONST row named (`macrosetup:40`).
	cpu 68000
	padding off
	phase 0
cnop	macro	offset,alignment
	org (*-1+(alignment)-((*-1+(-(offset)))#(alignment)))
	endm
align	macro	alignment
	cnop 0,alignment
	endm
	dc.b $11
	cnop 0,4
	dc.b $22
	cnop 2,8
	dc.b $33
	align 16
	dc.b $44
	end
