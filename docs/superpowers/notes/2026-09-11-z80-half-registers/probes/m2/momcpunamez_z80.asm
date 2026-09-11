	cpu z80
	org 0
	if MOMCPUNAME="Z80"
	db 1
	else
	db 2
	endif
	nop
	nop
	end
