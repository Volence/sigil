	cpu	68000
	padding off
	org	0
; DOMAIN census: what does the build do with an argument outside a function's
; mathematical domain? Expected to exit non-zero; read the DIAGNOSTICS, not the
; byte column. The corpus never asks any of these, so this decides only what
; sigil should do at the edges, not any shipped byte.
	dc.l	INT(LOG(0))
	dc.l	INT(LOG(-1))
	dc.l	INT(SQRT(-1))
	dc.l	INT(ASIN(2))
	dc.l	INT(ATANH(2))
	dc.l	INT(ACOSH(0))
	end
