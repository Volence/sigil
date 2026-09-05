	cpu	68000
	padding	off
A	equ	6
B	equ	2
C	equ	3
K	equ	3
J	equ	7
	org	0
; --- semantics: && / || are NORMALISING logical operators, not bitwise ------
	dc.b	6&&3
	dc.b	4&&2
	dc.b	4||2
	dc.b	0&&5
	dc.b	0||0
	dc.b	2=2
; --- && against every other tier -------------------------------------------
	dc.b	1&&12&3
	dc.b	6&4&&2
	dc.b	0&&8|4
	dc.b	4|0&&8
	dc.b	0&&8!4
	dc.b	4!0&&8
	dc.b	1&&1<<3
	dc.b	1<<3&&0
	dc.b	1&&2+3
	dc.b	3+0&&0
	dc.b	2&&3*4
	dc.b	1&&2=2
	dc.b	2=2&&1
	dc.b	7<>3&&0
	dc.b	5>1&&0
	dc.b	0<=1&&0
	dc.b	0>=1&&0
; --- || against every other tier -------------------------------------------
	dc.b	1||0&&0
	dc.b	0&&0||1
	dc.b	0||12&3
	dc.b	6&4||0
	dc.b	0||8|4
	dc.b	8|0||4
	dc.b	0||8!4
	dc.b	1||1<<3
	dc.b	1||2+3
	dc.b	1||2=2
	dc.b	2=2||0
; --- bitwise tier internal order, and against shifts / arithmetic ----------
	dc.b	1|2&2
	dc.b	1&6|3
	dc.b	1!3&2
	dc.b	3!1|2
	dc.b	3|1!3
	dc.b	1&3<<1
	dc.b	1&3>>1
	dc.b	1|3<<1
	dc.b	1!3<<1
	dc.b	1&3+1
	dc.b	1+3&2
	dc.b	3|2+2
	dc.b	1+3|4
	dc.b	3!2+2
	dc.b	1+3!2
	dc.b	3&2*3
	dc.b	3*2&5
	dc.b	3*2|5
	dc.b	3*2!5
	dc.b	3!2*2
; --- shifts against the arithmetic tier ------------------------------------
	dc.b	1+1<<3
	dc.b	1+8>>2
	dc.b	8-1<<2
	dc.b	2<<1*3
	dc.b	12/2<<1
	dc.b	12<<1/3
	dc.b	12#5<<1
	dc.b	8>>1<<2
; --- the multiplicative tier is one left-associative tier ------------------
	dc.b	7#5*2
	dc.b	12/2*3
	dc.b	12*2/3
	dc.b	12#5/2
	dc.b	6+2*3
; --- comparisons are ONE tier, and the loosest tier ------------------------
	dc.b	1<2=1
	dc.b	2=1<2
	dc.b	4=1+1
	dc.b	6&2=2
	dc.b	6<<1=12
	dc.b	3!1=2
	dc.b	1=3!2
; --- the rows this parcel was opened on, re-derived here -------------------
	dc.b	A=6&&C<>3
	dc.b	A=6||C=3
	dc.b	A&B=2
	dc.b	A|B=2
	dc.b	A>4&&B<5
	dc.b	A<<1=12
	dc.b	A+B*C
	dc.b	(K*2)=6&&(J<>3)
; --- the TYPED evaluator surface (int(...) routes through it) --------------
	dc.b	int(1&&2=2)
	dc.b	int(1+1<<3)
	dc.b	int(3!1|2)
	dc.b	int(2=2&&1)
	dc.b	int(1||2=2)
	dc.b	int(1.0+1<<3)
	dc.b	int(6.0/2*3)
