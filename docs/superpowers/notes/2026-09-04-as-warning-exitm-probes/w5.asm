	cpu 68000
	padding off
	org 0
	if 0
	warning "must not fire"
	endif
	warning "bare unquoted text"
	dc.b $11
