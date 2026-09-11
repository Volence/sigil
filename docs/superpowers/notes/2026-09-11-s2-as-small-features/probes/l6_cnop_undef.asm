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
	dc.b 1,2,3
	cnop -1,2<<lastbit(Nope)
	dc.b $00
	dc.b $EE
	end
