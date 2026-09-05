	cpu 68000
	padding off
	org 0
	message "SEEN pass=\{MOMPASS}"
	dc.b MOMPASS
K := L2-L1
	dc.b K
L1:	dc.w P-*
L2:	dc.b $BB
P:	dc.b $CC
	end
