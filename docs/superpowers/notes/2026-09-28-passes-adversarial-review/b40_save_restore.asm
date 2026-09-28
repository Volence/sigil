	cpu 68000
V set 1
	save
	if N=1
V set 2
	endif
	restore
	dc.b V
N equ 1
