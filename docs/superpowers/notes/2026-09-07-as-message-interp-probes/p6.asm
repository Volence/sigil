	cpu z80
	padding off
	org 0
	nop
	nop
	nop
	message "Uncompressed driver size: \{$}h bytes."
	message "here \{*}"
	message "both \{$} \{*}"
	db 1
	end
