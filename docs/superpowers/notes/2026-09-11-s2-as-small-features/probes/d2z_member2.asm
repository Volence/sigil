	cpu 68000
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying:	ds.b 1
zV ENDSTRUCT
	cpu z80
	org 0
	ld a,(zV.1upPlaying)
	ld a,(ix+zV.1upPlaying)
	db 0EEh
	end
