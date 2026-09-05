	cpu 68000
	padding off
	org 0
v42	equ 42
v255	equ 255
v4095	equ 4095
v10	equ 10
v4660	equ 4660
	message "d42=\{v42}"
	message "d255=\{v255}"
	message "d4095=\{v4095}"
	message "d10=\{v10}"
	message "d4660=\{v4660}"
	dc.b $11
	end
