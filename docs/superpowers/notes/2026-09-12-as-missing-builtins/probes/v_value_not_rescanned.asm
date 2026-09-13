	cpu 68000
	padding off
	org 0
n equ 5
s := "\\{n}"
	dc.b s,$EE
	dc.b substr(s,0,0),$EE
	dc.b "-"+s,$EE
	dc.b substr("\\{n}",0,0),$EE
t set s
	dc.b t,$EE
u set substr("\\{n}",0,0)
	dc.b u,$EE
	end
