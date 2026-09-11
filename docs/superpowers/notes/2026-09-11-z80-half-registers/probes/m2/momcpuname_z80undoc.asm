	cpu z80undoc
	org 0
	if MOMCPUNAME="Z80UNDOC"
	db 1
	else
	db 2
	endif
	nop
	nop
	end
