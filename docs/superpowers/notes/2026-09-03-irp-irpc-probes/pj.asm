	cpu 68000
	padding off
removeJmpTos = 0
extractJmpToName function name,val(substr(name, strstr(name, "_") + 1, strlen(name)))

    ; depending on if removeJmpTos is set or not, these macros will create a jump directly
    ; to the destination, or create a branch to a JmpTo
jsrto macro indirectaddr
	if removeJmpTos
		!jsr (extractJmpToName("indirectaddr")).l	; jump directly to address
	else
		!bsr.w indirectaddr	; otherwise, branch to an indirect JmpTo
	endif
    endm

jmpto macro indirectaddr
	if removeJmpTos
		!jmp (extractJmpToName("indirectaddr")).l	; jump directly to address
	else
		!bra.w indirectaddr	; otherwise, branch to an indirect JmpTo
	endif
    endm

jmpTosInternal2 macro
	if ARGCOUNT>0
	irp op,ALLARGS
op label *
	jmp	(extractJmpToName("op")).l
	endm
	endif
    endm

jmpTosInternal macro UseNop
	if ~~removeJmpTos
		if (*)&2
			; I wish I understood what really controls this.
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

	; Output list of JmpTos, pad start with a NOP instuction.
jmpTos macro
	jmpTosInternal TRUE,ALLARGS
	endm

	; Output list of JmpTos, pad start with zeroes.
jmpTos0 macro
	jmpTosInternal FALSE,ALLARGS
	endm

	org $1000
P:	equ $110000
Q:	equ $120000
R:	equ $130000
S:	equ $140000
T:	equ $150000
U:	equ $160000
	dc.w $A001
	jmpTos
	dc.w $A002
	jmpTos JmpTo_P
	dc.w $A003
	jmpTos JmpTo_Q,JmpTo_R,JmpTo_S
	dc.w $A004
	jmpTos0 JmpTo_T,JmpTo_U
	dc.w $A005
	end
