; Every nameless-label shape the Sonic 2 corpus uses, one per section.
; `/` is a DEFINITION-only form (a bidirectional nameless label); it is
; referenced with `+` or `-` like the others, never written as an operand.
	cpu	68000
	org	$2000
mybranch macro dest
	beq.s	dest
    endm
myentry macro ptr
	dc.w	ptr-Base
    endm
Base	label	*
	moveq	#$27,d0
; shape 1: the '/' bidirectional nameless label, reached backwards
/
	moveq	#$31,d1
	tst.b	d0
	bne.s	-
; shape 2: nameless label as a macro argument, branch
	tst.b	d0
	mybranch +
	moveq	#$32,d1
+
; shape 3: nameless label as a macro argument, data
	myentry +
	myentry (+)
; shape 4: nameless label as disp in disp(An,Xn)
	move.b	+(pc,d0.w),d1
+
	dc.b	$11,$22,$33,$44
	rts
