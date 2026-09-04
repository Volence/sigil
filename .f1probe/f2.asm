	cpu 68000
	padding off
; --- does int stay int inside INT()?  int-div -3 vs float floor -4
	dc.l INT(-7/2)
	dc.l INT(1/3*3)
	dc.l INT(3/2*2)
; --- float-typed symbols
fx = 3.7
	dc.l INT(fx)
	dc.l INT(fx+1)
fy equ 2.5
	dc.l INT(fy*2)
; --- mixed compare / bool
	dc.l INT(1.5<2)
	dc.l INT(2.5>2)
	dc.l 3.5<4
; --- builtins on floats
	dc.l INT(abs(-3.7))
	dc.l INT(sgn(-3.7))
	dc.l INT(sqrt(2.0)*1000)
	dc.l INT(CONSTPI*1000)
; --- precision: f64 vs 80-bit
	dc.l INT((0.1+0.2)*1000000000000000)
	dc.l INT(1e17+1-1e17)
	dc.l INT(123456789012345678.0/1000000000)
; --- the real thing
FM_Sample_Rate = 53267
	dc.l INT(15.39*1024*1024*2/FM_Sample_Rate+0.5)
	dc.l INT(29.15*1024*1024*2/FM_Sample_Rate+0.5)
PSG_Sample_Rate = 3546895
	dc.l INT(PSG_Sample_Rate/(130.98*2)+0.5)
	dc.l INT(PSG_Sample_Rate/(223721.56*2)+0.5)
	end
