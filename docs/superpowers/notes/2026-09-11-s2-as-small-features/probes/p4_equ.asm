	cpu 68000
	padding off
	org 0
E equ 5
	pushv ,E
	popv ,E
	dc.b E
	dc.b $EE
	end
