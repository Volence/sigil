; The same three undefined symbols, with the one unrelated error BELOW them.
; asl: 1 pass, 1 error, exit 2, warning present, undefined symbols reported ZERO.
;
; Identical to `error_first.asm` in outcome, which is the point: POSITION IS
; IRRELEVANT. What stops is the pass LOOP, not the reading of the file, so an
; error after the undefined symbols suppresses them exactly as one before them
; does. Any claim of the form "an EARLIER error swallows a LATER report" is a
; narrower rule than the one that holds.
	cpu	68000
	org	$1000
start:
	move.w	#UND_ONE,d0
	move.w	#UND_TWO,d1
	move.w	#UND_THREE,d2
	zzbogus	d0,d1
	rts
	end
