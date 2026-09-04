	cpu 68000
	padding off
	org $1000
kk:	equ 1<<zznotdefined
	dc.b $AA,kk,$BB
	end
