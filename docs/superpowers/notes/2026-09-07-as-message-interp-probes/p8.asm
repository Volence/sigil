	cpu 68000
	padding off
	org 0
fs equ 2.5
fi equ 1024.0
	message "fs2 \{fs*2}"
	message "fi2 \{fi/2}"
	message "gt \{3.5>4}"
	dc.b 1
	end
