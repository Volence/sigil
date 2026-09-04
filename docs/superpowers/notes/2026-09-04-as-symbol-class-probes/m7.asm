	cpu	68000
	padding	off
	org	0
; A forward branch forces asl to run more than one pass. Nothing here is a
; redefinition IN THE SOURCE; the question is whether re-executing the same
; `equ` and the same label on pass 2 counts as one.
; (`.w`, because sigil pins branch width and refuses an unsized `bra` — that
; refusal is unrelated to this parcel and would stop the probe before it reached
; the question.)
	bra.w	Fwd
Ap	equ	1
Lab:
	dc.w	Ap
Fwd:
	dc.w	$4444
