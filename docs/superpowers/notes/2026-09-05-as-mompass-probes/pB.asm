	cpu 68000
	padding off
	org 0
	message "SEEN pass=\{MOMPASS}"
	dc.b MOMPASS
	dc.w Later-*
Later:
	dc.b $EE
	end
