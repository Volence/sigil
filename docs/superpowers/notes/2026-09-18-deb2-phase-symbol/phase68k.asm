; phase68k.asm -- THE QUESTION: when a label sits inside a PHASE block, what
; address does asl print for it in its listing's symbol table -- the phase
; (VMA) address, or the physical (LMA) one?
;
; Shaped after the real case this measures: aeon's `soundbankhead.emp` section
; `(cpu: m68000, vma: $8000)` at map anchor lma $B8000, which the AS twin wrote
; as a `phase 08000h` bracket around the 5 engine-table heads.
;
; THE TWO CANDIDATE ANSWERS ARE FIVE DIGITS APART AND CANNOT BE MISREAD:
;   phase answer     PhasedHead : 8000
;   physical answer  PhasedHead : B8002
;
; THE CONTROLS are outside any PHASE bracket at addresses derivable by hand:
;   CtrlBefore : B8000   (org, first thing emitted)
;   CtrlAfter  : B8004   (org + 2 + 2, PHASE moves no physical bytes)
; A probe that listed nothing at all, or listed an empty table, is therefore
; distinguishable from one that answered `8000`: the controls would be gone too.
	cpu	68000
	org	$B8000
CtrlBefore:
	dc.w	$1111
	phase	$8000
PhasedHead:
	dc.w	$2222
	dephase
CtrlAfter:
	dc.w	$3333
	end
