	cpu	68000
	padding off
	org	0
; Third name census: the INTEGER-valued builtins the corpora actually write
; beside `abs`. `s1disasm/MacroSetup.asm(221)` uses `sgn(`, and
; `s1disasm/Macros.asm(353)` uses `abs(` inside a `rept` count. Expected to
; exit non-zero; read the diagnostics, and the values from `clean3.asm`.
	dc.l	SGN(-5)
	dc.l	SGN(0)
	dc.l	BITCNT(7)
	dc.l	FIRSTBIT(8)
	dc.l	LASTBIT(8)
	dc.l	BITPOS(8)
	dc.l	NOTBOGUS(1)
	end
