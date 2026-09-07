	cpu 68000
	padding off
	org 0
	message "fwd \{Later}"
	message "fwdf \{Later/1.0}"
	message "pass \{MOMPASS}"
	dc.b 1
	dc.w Later-*
Later:
	end
