	cpu 68000
	padding off
	org 0
n equ 5
N equ $AB
	dc.b strlen("\{n}"),$EE
	dc.b substr("\{n}xy",0,1),$EE
	dc.b lowstring("\{N}"),$EE
	dc.b substr("ab\{N}",2,1),$EE
	dc.b strlen("-"+"\{N}"),$EE
	end
