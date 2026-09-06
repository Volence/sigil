	cpu	68000
	padding off
	org	0
; What does INT() of a float too large for a 64-bit integer do? Decides whether
; sigil may keep `f.floor() as i64`, which SATURATES to i64::MAX in Rust.
; Expected to exit non-zero; read the diagnostics.
	dc.l	INT(1e30)
	dc.l	INT(1e30)-INT(1e30)
	dc.l	INT(EXP(1000))
	end
