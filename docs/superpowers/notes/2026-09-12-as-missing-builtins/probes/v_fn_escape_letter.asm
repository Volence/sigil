	cpu 68000
	padding off
	org 0
fq function n,"a\n b"
	dc.b fq(3),$EE
	end
