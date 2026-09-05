	cpu	68000
	padding	off
	org	0
; The census's own gap, put to asl. s2's `plrlistheader` writes `__LABEL__Plc:`
; and its `begin_animpat` writes `__LABEL___Blocks:`, and sigil substitutes the
; second and not the first — so one is a name the raw-body scan can claim and the
; other is not. Both spellings are here, in ONE `{INTLABEL}` body invoked TWICE
; under different captured labels, so "substituted" and "left literal" are
; different symbol names and the difference is visible in the table.
;
; The read is from OUTSIDE the expansion in both cases, which under the measured
; rule must be `#1010` for a PC label whichever spelling it ends up with — so the
; discriminator is not the read, it is which NAMES asl's own error text quotes.
mint	macro	{INTLABEL}
__LABEL__Plc:
	dc.w	$1111
__LABEL___Blocks:
	dc.w	$2222
	endm
Aint:	mint
Bint:	mint
	dc.w	AintPlc
	dc.w	Aint_Blocks
	dc.w	$4444
