	cpu 68000
	org 0
Rev:	equ 1
Lab:	if Rev=0
	dc.b 1
	else
	dc.b 2
	endif
	dc.l Lab+(1<<24)
