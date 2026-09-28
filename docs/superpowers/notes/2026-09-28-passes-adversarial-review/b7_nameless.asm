	cpu 68000
	bra.w +
	if N=1
	dc.w 0
	endif
+	nop
	bra.w -
N equ 1
