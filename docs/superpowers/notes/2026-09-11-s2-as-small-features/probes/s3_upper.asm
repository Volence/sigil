	cpu 68000
	padding off
	org 0
	ASL	$1A(a0)
	RoXr	(a1)
	dc.b $EE
	end
