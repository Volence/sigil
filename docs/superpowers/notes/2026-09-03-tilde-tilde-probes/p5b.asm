	cpu	68000
	padding off
; --- s2.macrosetup.asm's jmpTos family, with `extractJmpToName` inlined away
; --- (sigil cannot yet fold a user `function` returning a string into a `.l`
; --- absolute address; that gap is orthogonal to `~~` and is booked separately).
removeJmpTos = 0

jmpTosInternal2 macro
	if ARGCOUNT>0
	irp op,ALLARGS
op label *
	jmp	(op).l
	endm
	endif
    endm

jmpTosInternal macro UseNop
	if ~~removeJmpTos
		if (*)&2
			if UseNop
				nop
			else
				align 4
			endif
		endif

		shift

		jmpTosInternal2 ALLARGS

		align 4
	endif
    endm

jmpTos macro
	jmpTosInternal TRUE,ALLARGS
	endm

jmpTos0 macro
	jmpTosInternal FALSE,ALLARGS
	endm

	org	$1000
	rts
	jmpTos A,B
	dc.b	$22,$22
	jmpTos ; Empty
	dc.b	$33,$33,$33,$33,$33,$33
	jmpTos C
	dc.b	$44,$44
	jmpTos0 D
	dc.b $55,$55,$55,$55
	end
