	cpu 68000
	padding off
FM_Sample_Rate = 53267
roundFloatToInteger function float,INT(float+0.5)
MakeFMFrequency function frequency,roundFloatToInteger(frequency*1024*1024*2/FM_Sample_Rate)
	dc.w MakeFMFrequency(15.39)
	irp op, 15.39, 16.35
	dc.w MakeFMFrequency(op)+1*$800
	endm
	dc.w INT(3.7)
	dc.w min($3FF,roundFloatToInteger(3546895.0/(130.98*2)))
	end
