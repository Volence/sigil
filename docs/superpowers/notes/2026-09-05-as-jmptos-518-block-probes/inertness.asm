; The inertness claim, in falsifiable form.
;
; The claim behind "this parcel cannot change what a program that assembles
; today assembles to" is that a builtin head immediately before a `(`, in any
; 68000 instruction operand position OUTSIDE a held-back EA base group, is a
; hard refusal on master. Every line below is one such position. Run this file
; through a PRE-PARCEL sigil: every one of them must be diagnosed. A line that
; assembles is a counterexample and the claim is wrong.
;
; The last two lines are the exception the claim names: a builtin head used as
; an ordinary displacement symbol, where the `(a0)` IS a held-back EA base
; group. Those two must assemble on master and must still assemble after.
	cpu	68000
	org	$1000
val:	equ	4
strlen:	equ	4
Foo:
	jmp	(strlen("abcdefghijkl")+Foo).l
	jmp	(strlen("abcdefghijkl")+Foo).w
	jsr	(val("Foo")).l
	move.l	#strlen("abcdefghijkl"),d0
	move.l	#int(3.7),d0
	move.l	strlen("abcdefghijkl")+Foo,d0
	move.w	d0,(val("Foo")).l
	bra.w	strlen("abcdefghijkl")+Foo
	move.l	val(a0),d0
	move.l	strlen(a0,a0.l),d0
	end
