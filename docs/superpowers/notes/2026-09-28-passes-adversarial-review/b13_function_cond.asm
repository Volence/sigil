	cpu 68000
	ifdef A
ff	function x,x*2
	endif
	dc.b ff(3)
A = 1
