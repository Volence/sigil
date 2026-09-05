	cpu 68000
	padding off
	org 0
n1	equ -1
n42	equ -42
n255	equ -255
	message "m1=\{n1}"
	message "m42=\{n42}"
	message "m255=\{n255}"
	message "expr=\{0-1}"
	dc.b $11
	end
