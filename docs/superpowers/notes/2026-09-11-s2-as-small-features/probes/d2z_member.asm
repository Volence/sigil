	cpu z80
	org 0
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying:	ds.b 1
zV ENDSTRUCT
	ld a,(zV.1upPlaying)
	ld a,(ix+zV.1upPlaying)
	db 0EEh
	end
