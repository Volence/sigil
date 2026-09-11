	cpu 68000
	padding off
	org 0
org	macro address
	if address < *
	error "too much"
	elseif address > *
	!org address
	endif
	endm
cnop	macro offset,alignment
	org (*-1+(alignment)-((*-1+(-(offset)))#(alignment)))
	endm
Start:
	ds.b $123
	cnop -1,2<<lastbit(*-Start-1)
	dc.b $00
EndR:
	dc.l EndR
	dc.b $EE
	end
