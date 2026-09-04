	cpu 68000
	padding off
Base:
	dc.l	Fwd			; plain forward reference
	dc.l	Fwd+(2<<24)		; forward reference plus a large addend
	dc.l	(2<<24)|Fwd		; forward reference OR'd with a constant
	dc.l	Back+(2<<24)		; backward reference plus a large addend
	dc.l	(2<<24)|Back		; backward reference OR'd with a constant
Back:
	dc.w	0
Fwd:
	dc.w	0
	end
