	cpu	68000
	padding	off
	supmode	on

SmallW:	equ	$1000
BigL:	equ	$00A00000
BoundW:	equ	$7FFE
BoundL:	equ	$8000
HiW:	equ	$FF8000
HiTop:	equ	$FFFFFE

	org	$000000

; --- bare vs parenthesised, small (abs.w expected)
	move.w	SmallW,d0
	move.w	(SmallW),d0
	move.w	(SmallW).w,d0
; --- bare vs parenthesised, big (abs.l expected)
	move.w	BigL,d0
	move.w	(BigL),d0
; --- width boundaries
	move.w	BoundW,d0
	move.w	(BoundW),d0
	move.w	BoundL,d0
	move.w	(BoundL),d0
	move.w	HiW,d0
	move.w	(HiW),d0
	move.w	HiTop,d0
	move.w	(HiTop),d0
; --- register indirect must stay register indirect
	move.w	(a0),d0
	move.w	(a0)+,d0
	move.w	-(a0),d0
	move.w	(4,a0),d0
	move.w	4(a0),d0
; --- expression inside parens
	move.w	(SmallW+2),d0
	move.w	SmallW+2,d0
	move.w	((SmallW)),d0
; --- destination side
	move.w	d0,(SmallW)
	move.w	d0,SmallW
	move.w	d0,(BigL)
; --- other mnemonics
	lea	(SmallW),a1
	lea	SmallW,a1
	lea	(BigL),a1
	clr.w	(SmallW)
	tst.b	(BigL)
	move.b	(SmallW),(BigL)
; --- forward reference
	move.w	(FwdLab),d0
	move.w	FwdLab,d0
	move.l	(FwdLab),d1
FwdLab:	dc.w	0
	dc.w	0
