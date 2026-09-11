; A probe built from skdisasm's own text (2fcd861c): `notZ80` and the `org`
; macro from sonic3k.macrosetup.asm, the driver file's RAM phase blocks and
; rewind (Sound/Z80 Sound Driver.asm 18, 115, 199-227), and both driver exits
; (4433-4444 and 5313-5315), verbatim. Small Z80 bodies stand in for the
; driver and its data, and the two guesses are this probe's own.
	cpu 68000
	padding off

notZ80 function cpu,(cpu<>128)&&(cpu<>32988)

; make org safer (impossible to overwrite previously assembled bytes)
org macro address
	if notZ80(MOMCPU)
.diff := address - *
		if .diff < 0
			error "too much stuff before org $\{address} ($\{(-.diff)} bytes)"
		else
			while .diff > 1024
				; AS can only generate 1 kb of code on a single line
				dc.b [1024]$FF
.diff := .diff - 1024
			endm
			dc.b [.diff]$FF
		endif
	else
		if address < $
			error "too much stuff before org 0\{address}h (0\{($-address)}h bytes)"
		else
			while address > $
				db 0
			endm
		endif
	endif
    endm

Size_of_Snd_driver_guess = $A0
Size_of_Snd_driver2_guess = $60
zDataStart = $1C00

	!org 0
	dc.l 0,0
	dc.w $4E71
z80_SoundDriverStart:
		phase zDataStart
zSongBank:	ds.b 2
zTempVariablesStart:	ds.b $20
		dephase

		phase $1C40
zTracksSFXStart:	ds.b $30
		dephase
; ---------------------------------------------------------------------------
		!org z80_SoundDriverStart	; Rewind the ROM address to where we were earlier (allocating the RAM above messes with it)
; z80_SoundDriver:
Z80_SoundDriver:
		org Z80_SoundDriver+Size_of_Snd_driver_guess	; This 'org' inserts some padding that we can paste the compressed sound driver over later (see the 's3p2bin' tool)

		save
		!org 0	; z80 Align, handled by the build process
		CPU Z80
		listing purecode
		di
		im 1
		ld sp,1FFEh
		jp 38h
		org 38h
		ld a,(1C00h)
		or a
		jr z,38h
		db "SMPS Z80 driver stand-in", 0
		db 1,2,3,4,5,6,7,8,1,2,3,4,5,6,7,8,1,2,3,4,5,6,7,8
		restore
		padding off
		!org Z80_SoundDriver+Size_of_Snd_driver_guess	; The assembler still thinks we're in Z80 memory, so use an 'org' to switch back to the cartridge

; Z80_Snd_Driver2:
Z80_SoundDriverData:
		org Z80_SoundDriverData+Size_of_Snd_driver2_guess	; Once again, create some padding that we can paste the compressed data over later
; ---------------------------------------------------------------------------
		save
		CPU Z80
		listing purecode
		!org 1300h	; z80 Align, handled by the build process
		dw 1310h, 1320h, 1330h
		db 0F2h, 80h, 0E0h, 0F2h, 80h, 0E0h, 0F2h, 80h, 0E0h
		db "music data stand-in", 0
		restore
		padding off
		!org Z80_SoundDriverData+Size_of_Snd_driver2_guess	; The assembler still thinks we're in Z80 memory, so use an 'org' to switch back to the cartridge
		dc.w $4E75
