	cpu 68000
	padding off
	dc.b $AA
	CPU Z80
zqp_bogus_head
	db 2
	CPU 68000
	dc.b (zqp_bogus_head)&$FF
	end
