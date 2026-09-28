	cpu 68000
ifdebug	macro
	ifdef Dx
	dc.b $A1
	endif
	endm
	ifdebug
Lbl:	ifdebug
	dc.b 0
