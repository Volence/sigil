; d9 - a repeated bracket group on one operand.
	cpu	68000
	padding	off
	org	$1000
	dc.b	[2][3]$FF
	end
