	cpu 68000
	padding off
	org $100
Base:
mac:	macro frame,mappings
	dc.l	(frame<<24)|mappings
	endm
	mac	1,Fwd
	mac	0,Fwd
	dc.l	(1<<24)|Fwd
	org $200
Fwd:
	dc.w	0
	end
