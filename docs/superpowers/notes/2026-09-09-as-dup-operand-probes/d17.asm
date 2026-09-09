; d17 - the per-line size ceiling. MacroSetup.asm's `org0` macro says "AS can
; only generate 1 kb of code on a single line" and chunks at 1024, while
; sonic.asm writes `dcb.b $62A,$FF` (1578 bytes) through the `dcb` macro.
	cpu	68000
	padding	off
	org	$1000
	dc.b	[1024]$FF
	dc.b	[1025]$FE
	dc.b	[1596]$FD
	dc.b	[$62A]$FC
	end
