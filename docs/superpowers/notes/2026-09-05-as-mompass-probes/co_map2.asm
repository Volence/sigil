	cpu 68000
	padding off
	org 0
	if MOMPASS=1
	include "inc/c.asm"
	endif
	include "inc/b.asm"
	include "inc/d.asm"
	include "inc/e.asm"
	include "inc/f.asm"
	dc.b $11
	dc.w Later-*
Later:
	end
