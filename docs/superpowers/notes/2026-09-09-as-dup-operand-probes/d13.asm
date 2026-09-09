; d13 - MacroSetup.asm's own `dcb` macro shape: `[count]value` reached through
; a macro parameter substitution with a `.ATTRIBUTE` width.
	cpu	68000
	padding	off
	org	$1000
dcb	macro	count,value
	dc.ATTRIBUTE	[count]value
	endm
	dcb.b	3,$FF
	dcb.w	2,$1234
	dcb.b	4*2,$20
	end
