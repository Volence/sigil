	cpu 68000
	padding off
	org 0
v	equ 42
z	equ 0
	message "zero=\{z}"
	warning "w=\{v}"
	error "e=\{v}"
	fatal "f=\{v}"
	dc.b $11
	end
