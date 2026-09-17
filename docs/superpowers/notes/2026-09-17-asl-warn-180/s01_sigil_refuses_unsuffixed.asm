	cpu 68000
	padding off
	org 0
	move	(1).w,d0
	tst	(1).w
	clr	(1).w
	addq	#1,(1).w
