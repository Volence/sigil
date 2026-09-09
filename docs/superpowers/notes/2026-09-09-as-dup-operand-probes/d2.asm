; d2 - a `[count]value` operand mixed with plain operands in one comma list.
	cpu	68000
	padding	off
	org	$1000
	dc.b	$01,[3]$FF,$02
	dc.b	[2]$AA,[2]$BB
	dc.w	$0001,[2]$1234,$0002
	end
