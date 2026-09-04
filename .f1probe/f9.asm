	cpu 68000
	padding off
Z80_Clock = 3579545
pcmLoopCounterBase function sampleRate,baseCycles, 1+(Z80_Clock/(sampleRate)-(baseCycles)+(13/2))/13
dpcmLoopCounter function sampleRate, pcmLoopCounterBase(sampleRate,301/2)
SndDAC_Timpani	label *
.sample_rate = 10047
dac_sample_metadata macro label,sampleRateScale
sample_rate_scale := 1.0
    if "sampleRateScale"<>""
sample_rate_scale := sampleRateScale
    endif
	dc.b dpcmLoopCounter(int(label.sample_rate*sample_rate_scale))
    endm
	dac_sample_metadata SndDAC_Timpani
	dac_sample_metadata SndDAC_Timpani, 1.30
	dac_sample_metadata SndDAC_Timpani, 0.95
	end
