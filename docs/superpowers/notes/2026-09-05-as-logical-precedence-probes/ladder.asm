	cpu	68000
	padding	off
	org	0
; PROBE andand_vs_bitand_L
	dc.b	1&&12&3
	dc.b	(1&&12)&3
	dc.b	1&&(12&3)
; PROBE andand_vs_bitand_R
	dc.b	6&4&&2
	dc.b	(6&4)&&2
	dc.b	6&(4&&2)
; PROBE andand_vs_bitor_L
	dc.b	0&&8|4
	dc.b	(0&&8)|4
	dc.b	0&&(8|4)
; PROBE andand_vs_bitor_R
	dc.b	4|0&&8
	dc.b	(4|0)&&8
	dc.b	4|(0&&8)
; PROBE andand_vs_bitxor_L
	dc.b	0&&8!4
	dc.b	(0&&8)!4
	dc.b	0&&(8!4)
; PROBE andand_vs_bitxor_R
	dc.b	4!0&&8
	dc.b	(4!0)&&8
	dc.b	4!(0&&8)
; PROBE andand_vs_shl_L
	dc.b	1&&1<<3
	dc.b	(1&&1)<<3
	dc.b	1&&(1<<3)
; PROBE andand_vs_shl_R
	dc.b	1<<3&&0
	dc.b	(1<<3)&&0
	dc.b	1<<(3&&0)
; PROBE andand_vs_add_L
	dc.b	1&&2+3
	dc.b	(1&&2)+3
	dc.b	1&&(2+3)
; PROBE andand_vs_add_R
	dc.b	3+0&&0
	dc.b	(3+0)&&0
	dc.b	3+(0&&0)
; PROBE andand_vs_mul_L
	dc.b	2&&3*4
	dc.b	(2&&3)*4
	dc.b	2&&(3*4)
; PROBE andand_vs_mul_R
	dc.b	3*1&&0
	dc.b	(3*1)&&0
	dc.b	3*(1&&0)
; PROBE andand_vs_eq_L
	dc.b	1&&2=2
	dc.b	(1&&2)=2
	dc.b	1&&(2=2)
; PROBE andand_vs_eq_R
	dc.b	2=2&&1
	dc.b	(2=2)&&1
	dc.b	2=(2&&1)
; PROBE andand_vs_ne_R
	dc.b	7<>3&&0
	dc.b	(7<>3)&&0
	dc.b	7<>(3&&0)
; PROBE andand_vs_lt_R
	dc.b	1<5&&0
	dc.b	(1<5)&&0
	dc.b	1<(5&&0)
; PROBE oror_vs_andand_L
	dc.b	1||0&&0
	dc.b	(1||0)&&0
	dc.b	1||(0&&0)
; PROBE oror_vs_andand_R
	dc.b	0&&0||1
	dc.b	(0&&0)||1
	dc.b	0&&(0||1)
; PROBE oror_vs_bitand_L
	dc.b	0||12&3
	dc.b	(0||12)&3
	dc.b	0||(12&3)
; PROBE oror_vs_bitand_R
	dc.b	6&4||0
	dc.b	(6&4)||0
	dc.b	6&(4||0)
; PROBE oror_vs_bitor_L
	dc.b	0||8|4
	dc.b	(0||8)|4
	dc.b	0||(8|4)
; PROBE oror_vs_bitor_R
	dc.b	8|0||4
	dc.b	(8|0)||4
	dc.b	8|(0||4)
; PROBE oror_vs_shl_L
	dc.b	1||1<<3
	dc.b	(1||1)<<3
	dc.b	1||(1<<3)
; PROBE oror_vs_add_L
	dc.b	1||2+3
	dc.b	(1||2)+3
	dc.b	1||(2+3)
; PROBE oror_vs_eq_L
	dc.b	1||2=2
	dc.b	(1||2)=2
	dc.b	1||(2=2)
; PROBE oror_vs_eq_R
	dc.b	2=2||0
	dc.b	(2=2)||0
	dc.b	2=(2||0)
; PROBE bitand_vs_bitor
	dc.b	1|2&2
	dc.b	(1|2)&2
	dc.b	1|(2&2)
; PROBE bitxor_vs_bitand
	dc.b	1!3&2
	dc.b	(1!3)&2
	dc.b	1!(3&2)
; PROBE bitxor_vs_bitor
	dc.b	3!1|2
	dc.b	(3!1)|2
	dc.b	3!(1|2)
; PROBE bitand_vs_shl
	dc.b	1&3<<1
	dc.b	(1&3)<<1
	dc.b	1&(3<<1)
; PROBE shl_vs_add
	dc.b	1+1<<3
	dc.b	(1+1)<<3
	dc.b	1+(1<<3)
; PROBE shl_vs_mul
	dc.b	2<<1*3
	dc.b	(2<<1)*3
	dc.b	2<<(1*3)
; PROBE eq_vs_add
	dc.b	4=1+1
	dc.b	(4=1)+1
	dc.b	4=(1+1)
; PROBE eq_vs_bitand
	dc.b	6&2=2
	dc.b	(6&2)=2
	dc.b	6&(2=2)
; PROBE eq_vs_shl
	dc.b	6<<1=12
	dc.b	(6<<1)=12
	dc.b	6<<(1=12)
; PROBE lt_vs_eq
	dc.b	1<2=1
	dc.b	(1<2)=1
	dc.b	1<(2=1)
; PROBE mul_vs_add
	dc.b	6+2*3
	dc.b	(6+2)*3
	dc.b	6+(2*3)

