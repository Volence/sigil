	cpu 68000
	padding off
	org 0
	if MOMPASS=1
	include "inc/a.asm"
	endif
	include "inc/b.asm"
	if MOMPASS=1
	include "inc/c.asm"
	endif
	dc.b $11
	dc.w Later-*
Later:
	end
