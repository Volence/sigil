	cpu	68000
	padding	off
	org	0
; The `label` DIRECTIVE with `*` as its operand, read from OUTSIDE the macro
; body that declares it. m18/m19 and p11 measured `Al label $100` — a constant
; nowhere near the program counter. This asks the PC-VALUED spelling, which is
; the one whose answer a wrong rule can accidentally agree with.
;
; THE EXPANSION IS DELIBERATELY NOT AT ADDRESS ZERO. At org 0 with the macro
; first in the file, `*` is $0 — and $0 is also what a global default, an
; unresolved fixup, and a zero-filled relocation all produce, so that shape
; cannot tell any of them apart. Four bytes of filler put the expansion at $4.
;
; Invoked ONCE: m18 measured a second `label` on the same name as
; `#1000 symbol double defined` even with the value unchanged, so no
; two-instance version of this shape assembles. The discriminator is the
; ADDRESS instead.
	dc.w	$1111
	dc.w	$2222
mx	macro
Xl	label	*
	endm
	mx
	dc.l	Xl
	dc.w	$4444
