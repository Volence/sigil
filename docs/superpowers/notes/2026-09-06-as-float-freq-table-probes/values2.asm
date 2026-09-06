	cpu	68000
	padding off
	org	0
; The same measurements as `values.asm`, written WITHOUT `1e6`-style exponent
; literals so that the identical source can be handed to sigil. (sigil's lexer
; does not implement `1e3`/`1.5e2`/`.5`/`1.` -- see the "Left open" section of
; `2026-09-04-f1-as-float-semantics.md`; the corpus writes none of them.)
;
; This file is the one the regression tests quote, so it must exit 0.

; `hud_counter`, copied from `s2.asm(87675-87682)`, with the `moveq` that reads
; the counter back (`s2.asm(87595)` etc.). The counter is "total digits minus
; one", so these six immediates are the shipped bytes the whole parcel is about.
hud_counter macro {INTLABEL},number
__LABEL__ label *
.loop_counter = int(log(number)) ; Total digits minus one.
	dc.l number
    endm

Hud_100000:	hud_counter 100000
Hud_10000:	hud_counter 10000
Hud_1000:	hud_counter 1000
Hud_100:	hud_counter 100
Hud_10:		hud_counter 10
Hud_1:		hud_counter 1

	moveq	#Hud_100000.loop_counter,d6
	moveq	#Hud_10000.loop_counter,d6
	moveq	#Hud_1000.loop_counter,d6
	moveq	#Hud_100.loop_counter,d6
	moveq	#Hud_10.loop_counter,d6
	moveq	#Hud_1.loop_counter,d6

; The base, and the exact spelling of it.
	dc.l	INT(LOG(100))		; 2 base 10 / 4 natural
	dc.l	INT(LN(100))		; 4, the natural one under its own name
	dc.l	INT(LOG(1000))		; 3 exact log10 / 2 via ln(x)/ln(10)

; INT() on a negative.
	dc.l	INT(-3.2)			; -4 floor / -3 truncate
	dc.l	INT(LOG(0.5))			; -1 floor /  0 truncate
	dc.l	INT(LOG(0.5)*1000000)		; -301030 floor / -301029 truncate

; The rest of the surface.
	dc.l	INT(EXP(2)*1000000)		; e^x, not 2^x
	dc.l	INT(SQRT(2)*1000000)
	dc.l	INT(SIN(1)*1000000)		; radians, not degrees
	dc.l	INT(COS(1)*1000000)
	dc.l	INT(TAN(1)*1000000)
	dc.l	INT(ATAN(1)*1000000)
	dc.l	INT(ASIN(1)*1000000)
	dc.l	INT(ACOS(0)*1000000)
	dc.l	INT(SINH(1)*1000000)
	dc.l	INT(COSH(1)*1000000)
	dc.l	INT(TANH(1)*1000000)
	dc.l	INT(ASINH(1)*1000000)
	dc.l	INT(ACOSH(2)*1000000)
	dc.l	INT(ATANH(0.5)*1000000)
	dc.l	INT(ABS(-3.25)*1000000)
	dc.l	INT(ABS(-3))
	end
