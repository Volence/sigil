	cpu 68000
	padding off
FM_Sample_Rate = 53267
MakeFMFrequency function frequency,(frequency*1024*1024*2/FM_Sample_Rate)
	dc.w MakeFMFrequency(15.39)
	irp op, 15.39, 16.35
	dc.w MakeFMFrequency(op)+1*$800
	endm
	irp q, 1, 2
	dc.w q
	endm
	end
