	cpu 68000
	padding off
	org 0
	if MOMPASS=1
	warning "first-iteration warning"
	endif
	dc.b $11
	dc.w Later-*
Later:
	end
