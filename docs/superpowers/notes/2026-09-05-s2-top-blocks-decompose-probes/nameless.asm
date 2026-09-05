	cpu	68000
	org	$1000
Start:
	moveq	#$27,d0
	tst.b	d0
	beq.s	+
	moveq	#$41,d1
	bra.s	++
+
	moveq	#$42,d1
+
	moveq	#$43,d2
-
	subq.b	#1,d2
	bne.s	-
	rts
