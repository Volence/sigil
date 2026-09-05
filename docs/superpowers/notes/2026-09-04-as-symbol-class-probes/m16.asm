	cpu	68000
	padding	off
	org	0
; --- a second declaration the assembler never EXECUTES ---
; The control for the whole rule: if #1000 fired on the mere presence of two
; declarations rather than on two executions, this would be an error too.
Ai	equ	7
	if	0
Ai	equ	9
	endif
	dc.w	Ai
; --- and the executed twin of the same shape, which MUST be an error ---
Bi	equ	7
	if	1
Bi	equ	9
	endif
	dc.w	Bi
	dc.w	$4444
