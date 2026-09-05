	cpu 68000
	padding off
	org 0
	ifndef Later
	fatal "not defined on the first pass"
	endif
	dc.b $11
Later	equ 5
	end
