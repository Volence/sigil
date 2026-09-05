	cpu 68000
	padding off
	org 0
s	equ "abc"
c	equ 'z'
	message "str=\{s}"
	message "chr=\{c}"
	message "cat=\{s}-\{c}"
	dc.b $11
	end
