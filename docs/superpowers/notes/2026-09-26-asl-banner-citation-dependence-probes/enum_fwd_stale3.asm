; enum_fwd_stale.asm with THREE accepted bytes above the refused enum line
; instead of one. Paired with enum_fwd_stale.asm and enum_fwd_org40.asm: the
; byte `dc.b a,b` lists moves with what sits above the line. The run exits 2
; with "Additional necessary passes not started", so the byte is not a value.
	cpu 68000
	padding off
	org 0
	dc.b $5A,$33,$44
	enum a=fw,b
	dc.b a,b
fw EQU 4
