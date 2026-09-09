	cpu 68000
; The DAC-equate `switch` block lifted VERBATIM from Sonic 1's
; sound/_smps2asm_inc.asm lines 88-168, with SonicDriverVer set to 2 so the
; matching arm is the SECOND one. dTimpani is $83 in the `case 1` arm and $85 in
; the `case 2` arm, so the emitted byte says which arm ran.
SonicDriverVer = 2
use_s3_samples = 0
use_sk_samples = 0
use_s3d_samples = 0
	switch SonicDriverVer
		case 1
			enum		dKick=$81,dSnare,dTimpani
			enum		dHiTimpani=$88,dMidTimpani,dLowTimpani,dVLowTimpani
		case 2
			enum		dKick=$81,dSnare,dClap,dScratch,dTimpani,dHiTom,dVLowClap,dHiTimpani,dMidTimpani
			nextenum	dLowTimpani,dVLowTimpani,dMidTom,dLowTom,dFloorTom,dHiClap
			nextenum	dMidClap,dLowClap
		case 3
			enum		dSnareS3=$81,dHighTom,dMidTomS3,dLowTomS3,dFloorTomS3,dKickS3,dMuffledSnare
			nextenum	dCrashCymbal,dRideCymbal,dLowMetalHit,dMetalHit,dHighMetalHit
			nextenum	dHigherMetalHit,dMidMetalHit,dClapS3,dElectricHighTom
			nextenum	dElectricMidTom,dElectricLowTom,dElectricFloorTom
			nextenum	dTightSnare,dMidpitchSnare,dLooseSnare,dLooserSnare
			nextenum	dHiTimpaniS3,dLowTimpaniS3,dMidTimpaniS3,dQuickLooseSnare
			nextenum	dClick,dPowerKick,dQuickGlassCrash
			nextenum	dGlassCrashSnare,dGlassCrash,dGlassCrashKick,dQuietGlassCrash
			nextenum	dOddSnareKick,dKickExtraBass,dComeOn,dDanceSnare,dLooseKick
			nextenum	dModLooseKick,dWoo,dGo,dSnareGo,dPowerTom,dHiWoodBlock,dLowWoodBlock
			nextenum	dHiHitDrum,dLowHitDrum,dMetalCrashHit,dEchoedClapHit_S3
			nextenum	dLowerEchoedClapHit_S3,dHipHopHitKick,dHipHopHitPowerKick
			nextenum	dBassHey,dDanceStyleKick,dHipHopHitKick2,dHipHopHitKick3
			nextenum	dReverseFadingWind,dScratchS3,dLooseSnareNoise,dPowerKick2
			nextenum	dCrashingNoiseWoo,dQuickHit,dKickHey,dPowerKickHit
			nextenum	dLowPowerKickHit,dLowerPowerKickHit,dLowestPowerKickHit
		case 4
			enum		dSnareS3=$81,dHighTom,dMidTomS3,dLowTomS3,dFloorTomS3,dKickS3,dMuffledSnare
			nextenum	dCrashCymbal,dRideCymbal,dLowMetalHit,dMetalHit,dHighMetalHit
			nextenum	dHigherMetalHit,dMidMetalHit,dClapS3,dElectricHighTom
			nextenum	dElectricMidTom,dElectricLowTom,dElectricFloorTom
			nextenum	dTightSnare,dMidpitchSnare,dLooseSnare,dLooserSnare
			nextenum	dHiTimpaniS3,dLowTimpaniS3,dMidTimpaniS3,dQuickLooseSnare
			nextenum	dClick,dPowerKick,dQuickGlassCrash
			nextenum	dGlassCrashSnare,dGlassCrash,dGlassCrashKick,dQuietGlassCrash
			nextenum	dOddSnareKick,dKickExtraBass,dComeOn,dDanceSnare,dLooseKick
			nextenum	dModLooseKick,dWoo,dGo,dSnareGo,dPowerTom,dHiWoodBlock,dLowWoodBlock
			nextenum	dHiHitDrum,dLowHitDrum,dMetalCrashHit,dEchoedClapHit
			nextenum	dLowerEchoedClapHit,dHipHopHitKick,dHipHopHitPowerKick
			nextenum	dBassHey,dDanceStyleKick,dHipHopHitKick2,dHipHopHitKick3
			nextenum	dReverseFadingWind,dScratchS3,dLooseSnareNoise,dPowerKick2
			nextenum	dCrashingNoiseWoo,dQuickHit,dKickHey,dPowerKickHit
			nextenum	dLowPowerKickHit,dLowerPowerKickHit,dLowestPowerKickHit
		elsecase;SonicDriverVer>=5
			if (use_s3_samples<>0)||(use_sk_samples<>0)||(use_s3d_samples<>0)
				enum		dSnareS3=$81,dHighTom,dMidTomS3,dLowTomS3,dFloorTomS3,dKickS3,dMuffledSnare
				nextenum	dCrashCymbal,dRideCymbal,dLowMetalHit,dMetalHit,dHighMetalHit
				nextenum	dHigherMetalHit,dMidMetalHit,dClapS3,dElectricHighTom
				nextenum	dElectricMidTom,dElectricLowTom,dElectricFloorTom
				nextenum	dTightSnare,dMidpitchSnare,dLooseSnare,dLooserSnare
				nextenum	dHiTimpaniS3,dLowTimpaniS3,dMidTimpaniS3,dQuickLooseSnare
				nextenum	dClick,dPowerKick,dQuickGlassCrash
			endif
			if (use_s3_samples<>0)||(use_sk_samples<>0)
				nextenum	dGlassCrashSnare,dGlassCrash,dGlassCrashKick,dQuietGlassCrash
				nextenum	dOddSnareKick,dKickExtraBass,dComeOn,dDanceSnare,dLooseKick
				nextenum	dModLooseKick,dWoo,dGo,dSnareGo,dPowerTom,dHiWoodBlock,dLowWoodBlock
				nextenum	dHiHitDrum,dLowHitDrum,dMetalCrashHit,dEchoedClapHit
				nextenum	dLowerEchoedClapHit,dHipHopHitKick,dHipHopHitPowerKick
				nextenum	dBassHey,dDanceStyleKick,dHipHopHitKick2,dHipHopHitKick3
				nextenum	dReverseFadingWind,dScratchS3,dLooseSnareNoise,dPowerKick2
				nextenum	dCrashingNoiseWoo,dQuickHit,dKickHey,dPowerKickHit
				nextenum	dLowPowerKickHit,dLowerPowerKickHit,dLowestPowerKickHit
			endif
			; For conversions:
			if (use_s2_samples<>0)
				if (use_s3_samples<>0)||(use_sk_samples<>0)||(use_s3d_samples<>0)
					nextenum	dKick
				else
					enum		dKick=$81
				endif
				nextenum	dSnare,dClap,dScratch,dTimpani,dHiTom,dVLowClap,dHiTimpani,dMidTimpani
				nextenum	dLowTimpani,dVLowTimpani,dMidTom,dLowTom,dFloorTom,dHiClap
				nextenum	dMidClap,dLowClap
			endif
			if (use_s3d_samples<>0)
				nextenum	dFinalFightMetalCrash,dIntroKick
			endif
			if (use_s3_samples<>0)
				nextenum	dEchoedClapHit_S3,dLowerEchoedClapHit_S3
			endif
	endcase
	dc.b dKick,dSnare,dClap,dScratch,dTimpani
	end
