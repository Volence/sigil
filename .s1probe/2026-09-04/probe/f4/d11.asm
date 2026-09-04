	cpu 68000
	padding off
vdp_ctrl equ $12
widthsel equ $34
ph:	macro loc,slot=(vdp_ctrl).l
	dc.b loc
	dc.b vdp_ctrl
	dc.b widthsel
	dc.b "<slot>"
	endm
	ph $77
	end
