	cpu 68000
	padding off
	org 0
v equ 42
	message "msg \{v}"
	warning "warn \{v}"
	dc.b $11
