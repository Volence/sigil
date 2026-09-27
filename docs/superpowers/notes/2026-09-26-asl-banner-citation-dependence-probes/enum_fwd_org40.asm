; 2026-09-04-as-enum-probes/q10.asm with the origin moved to $40 and nothing
; emitted above the refused enum line. Paired with enum_fwd_stale.asm and
; enum_fwd_stale3.asm. The run exits 2 with "Additional necessary passes not
; started", so the byte `dc.b a,b` lists is not a value.
	cpu 68000
	padding off
	org $40
	enum a=fw,b
	dc.b a,b
fw EQU 4
