	cpu 68000
	padding off
	org 0
long	equ "0123456789012345678901234567890123456789ab"
	message "strlen=\{strlen(long)}"
	message "nested=\{strlen(long)*1}"
	dc.b $11
	end
