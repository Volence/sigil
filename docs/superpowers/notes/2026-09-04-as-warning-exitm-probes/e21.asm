	cpu 68000
	padding off
	phase 0
clearRAM macro startaddr,endaddr
    if startaddr>endaddr
	fatal "Starting address of clearRAM \{startaddr} is after ending address \{endaddr}."
    elseif startaddr==endaddr
	warning "clearRAM is clearing zero bytes. Turning this into a nop instead."
	exitm
    endif
	dc.b $C0
	endm
	dc.b $11
	clearRAM 4,8
	dc.b $22
	clearRAM 8,8
	dc.b $33
