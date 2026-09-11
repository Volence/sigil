	cpu z80
	org 0
pal	macro path
	db "path"
	endm
	pal Special Stage 1 2p.bin
	db 0EEh
	end
