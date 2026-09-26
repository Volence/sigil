; 2026-09-04-as-macro-default-params.md, "Logged, not fixed": asl reads a
; missing operand as absolute address zero. The first two lines are the note's
; own shapes; the last two put an accepted value directly above each, so a
; substituted (carried-over) value would show as $7777 / $5678 instead of 0.
	cpu 68000
	padding off
	org 0
	move.l	#$1234,
	move.l	,d0
	dc.w	$7777
	move.l	,d0
	move.l	#$5678,
