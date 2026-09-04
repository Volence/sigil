; Fixture for `as_end_directive.rs`. `end` inside the INCLUDED file ends the
; whole assembly unit, so neither `part.asm`'s trailing `dc.b $99` nor this
; file's own `dc.b $44` after the `include` is assembled. The `end` in the
; FALSE `if` arm below is not executed and must not stop anything.
;
; asl 1.42 Beta Bld 212 image: 11 22 33. Probe: end2.asm in
; docs/superpowers/notes/2026-09-04-as-end-probes/.
	cpu 68000
	padding off
	phase 0
	dc.b $11
	if 0
	end
	endif
	dc.b $22
	include "part.asm"
	dc.b $44
