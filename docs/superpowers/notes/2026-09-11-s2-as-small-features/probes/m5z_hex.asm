	cpu z80
	org 0
pal	macro path
	db "path"
	endm
	pal 0101b 1Fh 0FFh
	db 0EEh
	end
