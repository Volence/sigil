; A `dc.<size>` whose operand is a bare register.
;
; This is the shape that makes the s2disasm build (md5 0dee1f98…) ABORT:
;
;   asl: /home/runner/work/asl-releases/asl-releases/motpseudo.c:969:
;        DecodeMotoDC: Assertion `0' failed.
;   Aborted (core dumped)                          exit 134
;
; The other three builds have that assertion compiled out. They emit NO BYTES
; for the line, advance the location counter by nothing, print no diagnostic and
; exit 0 — so everything after it moves, silently.
;
; The assertion is the evidence that matters: upstream marked this case
; UNREACHABLE. `DecodeMotoDC`'s switch over the operand's type handles TempInt,
; TempFloat, TempString and TempNone, and `default: assert(0)`. TempReg is not a
; case, because a register is not a thing AS's author expected to arrive here.
	cpu 68000
	padding off
	org $1000
	move.w	#$A101,d0
	dc.w	a1
	move.w	#$A202,d0
