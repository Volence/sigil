	cpu 68000
	padding off
	org 0
mw macro n
	if n==0
	warning "zero seen"
	exitm
	endif
	dc.b n
	endm
	mw 1
	mw 0
	mw 2
