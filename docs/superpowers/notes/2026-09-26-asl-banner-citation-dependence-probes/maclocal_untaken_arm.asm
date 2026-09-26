; 2026-09-03-as-maclocal-scope.md, "What it costs: the untaken arm": one
; written branch, two destinations. `mcond 0` reaches the caller's .done
; (67FE at PC 2), `mcond 1` its own (6704). Reconstructed; the note's own probe
; file was not committed. Both calls sit at the same PC in separate scopes.
	cpu 68000
	padding off
	org 0
mcond	macro	c
	beq.s	.done
	if c
	nop
	nop
.done:
	endif
	endm
Base:
	dc.w	Later-Base
.done:
	mcond	0
	nop
	nop
	nop
Other:
	dc.w	Later-Other
.done:
	mcond	1
Later:
	nop
