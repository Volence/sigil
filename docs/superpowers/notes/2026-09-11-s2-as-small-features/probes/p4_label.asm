	cpu 68000
	padding off
	org 0
Lbl:	dc.b 1
	pushv ,Lbl
	popv ,Lbl
	dc.b $EE
	end
