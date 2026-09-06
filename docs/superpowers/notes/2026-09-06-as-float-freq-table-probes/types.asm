	cpu	68000
	padding off
	org	0
; TYPE census. Every line is expected to be an `error #1133/#1134` or a value;
; this file exits non-zero BY DESIGN and is not a source of byte values --
; only of which lines asl refused and with which error number.
	dc.l	LOG(100)	; float result in an int slot?
	dc.l	ABS(-3)		; does ABS preserve the INTEGER type?
	dc.l	ABS(-3.25)	; float in, float out?
	dc.l	SQRT(16)	; exact result -- still a float?
	dc.l	LOG(100)&1	; float into a bitwise op
	end
