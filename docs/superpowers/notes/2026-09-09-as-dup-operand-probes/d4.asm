; d4 - whitespace inside and after the bracket group.
	cpu	68000
	padding	off
	org	$1000
	dc.b	[ 3 ]$AA
	dc.b	[3] $BB
	dc.b	[ 2 ] $CC
	end
