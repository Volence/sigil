	cpu 68000
	padding off
FM_Sample_Rate = 53267
PSG_Sample_Rate = 3546895
roundFloatToInteger function float,INT(float+0.5)
minf function a,b,b!((a!b)&(-(a<b)))
MakeFMFrequency function frequency,roundFloatToInteger(frequency*1024*1024*2/FM_Sample_Rate)
MakePSGFrequency function frequency,minf($3FF,roundFloatToInteger(PSG_Sample_Rate/(frequency*2)))
	irp op, 15.39, 16.35, 17.34, 18.36, 19.45, 20.64, 21.84, 23.13, 24.51, 25.98, 27.53, 29.15
	dc.w MakeFMFrequency(op)+1*$800
	endm
	irp op, 130.98, 138.78, 223721.56, 6991.28, 4142.98
	dc.w MakePSGFrequency(op)
	endm
	dc.l INT(3.7),INT(-3.7),INT(-3.2),INT(-3.0),INT(7),int(3.7)
	dc.l INT(2.5+0.5),INT(-2.5+0.5),INT(-3.5+0.5)
	dc.l INT(-7/2),INT(1/3*3),INT(3/2*2)
fx = 3.7
	dc.l INT(fx),INT(fx+1)
fy equ 2.5
	dc.l INT(fy*2)
	dc.l INT(1.5<2),INT(2.5>2),3.5<4
	dc.l 7/2,INT(7/2),-7/2,1/3*3,INT(7.0/2),INT(1.0*3)
	dc.w INT(3.7)
	dc.b INT(3.7)
	dc.b 0
sc := 1.0
	dc.b INT(100*sc)
sc := 1.30
	dc.b INT(100*sc)
	end
