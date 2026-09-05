	cpu	68000
	padding	off
	org	0
; PROBE shl_vs_div
	dc.b	12/2<<1
	dc.b	(12/2)<<1
	dc.b	12/(2<<1)
; PROBE div_vs_shl
	dc.b	12<<1/3
	dc.b	(12<<1)/3
	dc.b	12<<(1/3)
; PROBE shr_vs_add
	dc.b	1+8>>2
	dc.b	(1+8)>>2
	dc.b	1+(8>>2)
; PROBE sub_vs_shl
	dc.b	8-1<<2
	dc.b	(8-1)<<2
	dc.b	8-(1<<2)
; PROBE shl_vs_shr
	dc.b	8>>1<<2
	dc.b	(8>>1)<<2
	dc.b	8>>(1<<2)
; PROBE mod_vs_shl
	dc.b	12#5<<1
	dc.b	(12#5)<<1
	dc.b	12#(5<<1)
; PROBE mod_vs_mul
	dc.b	7#5*2
	dc.b	(7#5)*2
	dc.b	7#(5*2)
; PROBE bitand_vs_add
	dc.b	1&3+1
	dc.b	(1&3)+1
	dc.b	1&(3+1)
; PROBE add_vs_bitand
	dc.b	1+3&2
	dc.b	(1+3)&2
	dc.b	1+(3&2)
; PROBE bitor_vs_add
	dc.b	3|2+2
	dc.b	(3|2)+2
	dc.b	3|(2+2)
; PROBE bitxor_vs_add
	dc.b	3!2+2
	dc.b	(3!2)+2
	dc.b	3!(2+2)
; PROBE bitand_vs_mul
	dc.b	3&2*3
	dc.b	(3&2)*3
	dc.b	3&(2*3)
; PROBE add_vs_bitor
	dc.b	1+3|4
	dc.b	(1+3)|4
	dc.b	1+(3|4)
; PROBE eq_vs_bitxor
	dc.b	3!1=2
	dc.b	(3!1)=2
	dc.b	3!(1=2)
; PROBE oror_vs_bitxor_L
	dc.b	0||8!4
	dc.b	(0||8)!4
	dc.b	0||(8!4)
; PROBE eq_vs_lt_R
	dc.b	2=1<2
	dc.b	(2=1)<2
	dc.b	2=(1<2)
; PROBE andand_vs_gt_R
	dc.b	5>1&&0
	dc.b	(5>1)&&0
	dc.b	5>(1&&0)
; PROBE andand_vs_le_R
	dc.b	0<=1&&0
	dc.b	(0<=1)&&0
	dc.b	0<=(1&&0)
; PROBE andand_vs_ge_R
	dc.b	0>=1&&0
	dc.b	(0>=1)&&0
	dc.b	0>=(1&&0)
; PROBE andand_vs_lt_R2
	dc.b	0<1&&0
	dc.b	(0<1)&&0
	dc.b	0<(1&&0)
; PROBE andand_assoc
	dc.b	1&&0&&1
	dc.b	(1&&0)&&1
	dc.b	1&&(0&&1)
; PROBE oror_assoc
	dc.b	0||1||0
	dc.b	(0||1)||0
	dc.b	0||(1||0)

