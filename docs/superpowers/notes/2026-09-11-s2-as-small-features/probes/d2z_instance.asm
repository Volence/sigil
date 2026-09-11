	cpu z80
	org 0
zV STRUCT DOTS
	A:		ds.b 1
	1upPlaying:	ds.b 1
zV ENDSTRUCT
	db 11h
zAbs:	zV
	ld a,(zAbs.1upPlaying)
	db 0EEh
	end
