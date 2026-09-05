	cpu 68000
	padding off
	org 0
c	equ 'z'
d	equ 'z'+0
e	equ "abc"
	message "chr_expr=\{c+0}"
	message "chr_plain=\{d}"
	message "strlen=\{strlen(e)}"
	message "sub=\{substr(e,1,2)}"
	message "lit=\{'z'}"
	message "litnum=\{'z'+1}"
	dc.b $11
	end
