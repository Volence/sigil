	cpu 68000
	padding off
	org 0
S := "ab"
	pushv ,S
S := "cd"
	dc.b S
	popv ,S
	dc.b S
	dc.b $EE
	end
