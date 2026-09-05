	cpu 68000
	padding off
	org 0
	message "SEEN pass=\{MOMPASS}"
	dc.b MOMPASS
V := W
	dc.b V
W:	dc.b $EE
	end
