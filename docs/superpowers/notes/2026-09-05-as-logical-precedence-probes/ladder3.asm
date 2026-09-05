	cpu	68000
	padding	off
	org	0
; PROBE mul_vs_div
	dc.b	12/2*3
	dc.b	(12/2)*3
	dc.b	12/(2*3)
; PROBE div_vs_mul
	dc.b	12*2/3
	dc.b	(12*2)/3
	dc.b	12*(2/3)
; PROBE mod_vs_div
	dc.b	12#5/2
	dc.b	(12#5)/2
	dc.b	12#(5/2)
; PROBE bitand_vs_mul_R
	dc.b	3*2&5
	dc.b	(3*2)&5
	dc.b	3*(2&5)
; PROBE bitor_vs_mul_R
	dc.b	3*2|5
	dc.b	(3*2)|5
	dc.b	3*(2|5)
; PROBE bitxor_vs_mul_R
	dc.b	3*2!5
	dc.b	(3*2)!5
	dc.b	3*(2!5)
; PROBE bitxor_vs_mul_L
	dc.b	3!2*2
	dc.b	(3!2)*2
	dc.b	3!(2*2)
; PROBE add_vs_bitxor
	dc.b	1+3!2
	dc.b	(1+3)!2
	dc.b	1+(3!2)
; PROBE eq_vs_bitxor_R
	dc.b	1=3!2
	dc.b	(1=3)!2
	dc.b	1=(3!2)
; PROBE bitand_vs_shr
	dc.b	1&3>>1
	dc.b	(1&3)>>1
	dc.b	1&(3>>1)
; PROBE bitor_vs_shl
	dc.b	1|3<<1
	dc.b	(1|3)<<1
	dc.b	1|(3<<1)
; PROBE bitxor_vs_shl
	dc.b	1!3<<1
	dc.b	(1!3)<<1
	dc.b	1!(3<<1)
; PROBE bitand_vs_bitor_R
	dc.b	1&6|3
	dc.b	(1&6)|3
	dc.b	1&(6|3)
; PROBE bitor_vs_bitxor_R
	dc.b	3|1!3
	dc.b	(3|1)!3
	dc.b	3|(1!3)

