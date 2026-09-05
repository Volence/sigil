	cpu 68000
	padding off
	org 0
	dc.b $11
	rept 41
	dc.b $22
	endm
here:
	dc.b $99
	message "lbl=\{here}"
	message "pc=\{*}"
	message "sum=\{here+here}"
	end
