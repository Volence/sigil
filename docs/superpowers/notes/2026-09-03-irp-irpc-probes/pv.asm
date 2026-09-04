	cpu 68000
	padding off
	org $1000
bb:	equ 1<<zzundef
mk := 0
mk := mk|bb
	dc.b $AA,mk,$BB
	end
