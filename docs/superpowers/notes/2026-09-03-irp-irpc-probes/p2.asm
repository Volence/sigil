	cpu 68000
	padding off
	org $1000
; 2a: one param, three args, ARGCOUNT before/after each shift
one	macro pp
	dc.b ARGCOUNT
	shift
	dc.b ARGCOUNT
	shift
	dc.b ARGCOUNT
	shift
	dc.b ARGCOUNT
	endm
	one 11,22,33
; 2b: three params, three args
three	macro a,b,c
	dc.b $EE,ARGCOUNT
	shift
	dc.b ARGCOUNT
	shift
	dc.b ARGCOUNT
	shift
	dc.b ARGCOUNT
	endm
	three 11,22,33
; 2c: three params, one arg
	dc.b $DD
	three 11
; 2d: no args at all
zero	macro
	dc.b $CC,ARGCOUNT
	endm
	zero
; 2e: ARGCOUNT outside a macro
	dc.b $BB,ARGCOUNT
	end
