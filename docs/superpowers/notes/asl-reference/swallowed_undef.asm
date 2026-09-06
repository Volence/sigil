; swallowed_undef.asm - a run that FAILS, and reports nothing about a whole
; class of defect it contains.
;
; Three symbols here are undefined and are never defined below: UND_ONE,
; UND_TWO, UND_THREE. asl reports NONE of them. It reports the one unrelated
; error, exits 2, and its footer reads `1 error`.
;
; `swallowed_undef_control.asm` beside this file is this file MINUS the
; `zzbogus` line and nothing else. It reports all three, exits 2 as well, and
; its footer reads `3 errors`. So both arms fail: the pair does not separate
; "asl succeeded" from "asl failed", it separates "asl looked" from "asl never
; looked", which the exit status cannot.
;
; The mechanism is the PASS LOOP. A forward reference is legal, so an undefined
; symbol is a provisional value on pass 1 and becomes an error only when a later
; pass finds it still undefined. Any error stops the loop, and that pass never
; runs. asl prints the fact at the bottom of the listing:
;
;     1 pass
;       Additional necessary passes not started due to
;       errors, listing possibly incorrect.
;
; POSITION IS IRRELEVANT. The `zzbogus` line is first here; moved below all
; three undefined symbols it suppresses them just the same, because what stops
; is the loop and not the reading of the file.
	cpu	68000
	org	$1000
start:
	zzbogus	d0,d1
	move.w	#UND_ONE,d0
	move.w	#UND_TWO,d1
	move.w	#UND_THREE,d2
	rts
	end
