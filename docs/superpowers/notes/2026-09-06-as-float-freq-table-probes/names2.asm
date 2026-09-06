	cpu	68000
	padding off
	org	0
; The rest of the candidate names. Every line here is EXPECTED to be either a
; value or `error #1860: unknown function`; this file is a NAME census, not a
; source of byte values, and it exits non-zero by design.
	dc.l	INT(ASIN(0))
	dc.l	INT(ACOS(1))
	dc.l	INT(ARCSIN(0))
	dc.l	INT(ARCTAN(0))
	dc.l	INT(SINH(0))
	dc.l	INT(COSH(0))
	dc.l	INT(TANH(0))
	dc.l	INT(ASINH(0))
	dc.l	INT(ACOSH(1))
	dc.l	INT(ATANH(0))
	dc.l	INT(EXP2(3))
	dc.l	INT(POW(2,3))
	dc.l	INT(SIGN(-5))
	dc.l	INT(BITCNT(7))
	dc.l	INT(FIRSTBIT(8))
	dc.l	INT(LASTBIT(8))
	dc.l	INT(BITPOS(8))
	dc.l	INT(TOUPPER(65))
	dc.l	INT(FRAC(1.5))
	dc.l	INT(TRUNC(1.5))
	dc.l	INT(ROUND(1.5))
	dc.l	INT(FLOOR(1.5))
	dc.l	INT(CEIL(1.5))
	end
