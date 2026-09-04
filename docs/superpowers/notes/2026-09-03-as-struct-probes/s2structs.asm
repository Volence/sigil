	cpu 68000
	padding off
	org $0
HorizontalScrollBuffer struct dots
	ds.l	224	; Total lines on the screen.
	ds.l	16	; A bug/optimisation in 'Swscrl_CPZ' causes these values to be overflowed into.
	ds.b	$40	; These are just unused.
HorizontalScrollBuffer endstruct

inst_HorizontalScrollBuffer:	HorizontalScrollBuffer
SoundQueue STRUCT DOTS
	Music0:	ds.b	1
	SFX0:	ds.b	1
	SFX1:	ds.b	1
	SFX2:	ds.b	1 ; This one is never used, since nothing ever gets written to it.
	Music1:	ds.b	1
SoundQueue ENDSTRUCT

inst_SoundQueue:	SoundQueue
zTrack STRUCT DOTS
	; 	"playback control"; bits:
	; 	1 (02h): track is at rest
	; 	2 (04h): SFX is overriding this track
	; 	3 (08h): modulation on
	; 	4 (10h): do not attack next note
	; 	7 (80h): track is playing
	PlaybackControl:	ds.b 1
	; 	"voice control"; bits:
	; 	2 (04h): If set, bound for part II, otherwise 0 (see zWriteFMIorII)
	; 		-- bit 2 has to do with sending key on/off, which uses this differentiation bit directly
	; 	7 (80h): PSG track
	VoiceControl:		ds.b 1
	TempoDivider:		ds.b 1	; Timing divisor; 1 = Normal, 2 = Half, 3 = Third...
	DataPointerLow:		ds.b 1	; Track's position low byte
	DataPointerHigh:	ds.b 1	; Track's position high byte
	Transpose:		ds.b 1	; Transpose (from coord flag E9)
	Volume:			ds.b 1	; Channel volume (only applied at voice changes)
	AMSFMSPan:		ds.b 1	; Panning / AMS / FMS settings
	VoiceIndex:		ds.b 1	; Current voice in use OR current PSG tone
	VolFlutter:		ds.b 1	; PSG flutter (dynamically affects PSG volume for decay effects)
	StackPointer:		ds.b 1	; "Gosub" stack position offset (starts at 2Ah, i.e. end of track, and each jump decrements by 2)
	DurationTimeout:	ds.b 1	; Current duration timeout; counting down to zero
	SavedDuration:		ds.b 1	; Last set duration (if a note follows a note, this is reapplied to 0Bh)
	;
	; 	; 0Dh / 0Eh change a little depending on track -- essentially they hold data relevant to the next note to play
	SavedDAC:			; DAC: Next drum to play
	FreqLow:		ds.b 1	; FM/PSG: frequency low byte
	FreqHigh:		ds.b 1	; FM/PSG: frequency high byte
	NoteFillTimeout:	ds.b 1	; Currently set note fill; counts down to zero and then cuts off note
	NoteFillMaster:		ds.b 1	; Reset value for current note fill
	ModulationPtrLow:	ds.b 1	; Low byte of address of current modulation setting
	ModulationPtrHigh:	ds.b 1	; High byte of address of current modulation setting
	ModulationWait:		ds.b 1	; Wait for ww period of time before modulation starts
	ModulationSpeed:	ds.b 1	; Modulation speed
	ModulationDelta:	ds.b 1	; Modulation change per mod. Step
	ModulationSteps:	ds.b 1	; Number of steps in modulation (divided by 2)
	ModulationValLow:	ds.b 1	; Current modulation value low byte
	ModulationValHigh:	ds.b 1	; Current modulation value high byte
	Detune:			ds.b 1	; Set by detune coord flag E1; used to add directly to FM/PSG frequency
	VolTLMask:		ds.b 1	; zVolTLMaskTbl value set during voice setting (value based on algorithm indexing zGain table)
	PSGNoise:		ds.b 1	; PSG noise setting
	VoicePtrLow:		ds.b 1	; Low byte of custom voice table (for SFX)
	VoicePtrHigh:		ds.b 1	; High byte of custom voice table (for SFX)
	TLPtrLow:		ds.b 1	; Low byte of where TL bytes of current voice begin (set during voice setting)
	TLPtrHigh:		ds.b 1	; High byte of where TL bytes of current voice begin (set during voice setting)
	LoopCounters:		ds.b $A	; Loop counter index 0
	;   ... open ...
	GoSubStack:			; start of next track, every two bytes below this is a coord flag "gosub" (F8h) return stack
	;
	;	The bytes between +20h and +29h are "open"; starting at +20h and going up are possible loop counters
	;	(for coord flag F7) while +2Ah going down (never AT 2Ah though) are stacked return addresses going
	;	down after calling coord flag F8h.  Of course, this does mean collisions are possible with either
	;	or other track memory if you're not careful with these!  No range checking is performed!
	;
	; 	All tracks are 2Ah bytes long
zTrack ENDSTRUCT

inst_zTrack:	zTrack
zVar STRUCT DOTS
	SFXPriorityVal:		ds.b 1
	TempoTimeout:		ds.b 1
	CurrentTempo:		ds.b 1	; Stores current tempo value here
	StopMusic:		ds.b 1	; Set to 7Fh to pause music, set to 80h to unpause. Otherwise 00h
	FadeOutCounter:		ds.b 1
	FadeOutDelay:		ds.b 1
	Communication:		ds.b 1	; Unused byte used to synchronise gameplay events with music
	DACUpdating:		ds.b 1	; Set to FFh while DAC is updating, then back to 00h
	QueueToPlay:		ds.b 1	; The head of the queue
	Queue0:			ds.b 1
	Queue1:			ds.b 1
	Queue2:			ds.b 1	; This slot was totally broken in Sonic 1's driver. It's mostly fixed here, but it's still a little broken (see 'zInitMusicPlayback').
	VoiceTblPtr:		ds.b 2	; Address of the voices
	FadeInFlag:		ds.b 1
	FadeInDelay:		ds.b 1
	FadeInCounter:		ds.b 1
	1upPlaying:		ds.b 1
	TempoMod:		ds.b 1
	TempoTurbo:		ds.b 1	; Stores the tempo if speed shoes are acquired (or 7Bh is played otherwise)
	SpeedUpFlag:		ds.b 1
	DACEnabled:		ds.b 1
	MusicBankNumber:	ds.b 1
	IsPalFlag:		ds.b 1	; Flags if the system is a PAL console
zVar ENDSTRUCT

inst_zVar:	zVar
	dc.l	HorizontalScrollBuffer.len
	dc.l	SoundQueue.Music0
	dc.l	SoundQueue.SFX0
	dc.l	SoundQueue.SFX1
	dc.l	SoundQueue.SFX2
	dc.l	SoundQueue.Music1
	dc.l	SoundQueue.len
	dc.l	inst_SoundQueue.Music0
	dc.l	inst_SoundQueue.SFX0
	dc.l	inst_SoundQueue.SFX1
	dc.l	inst_SoundQueue.SFX2
	dc.l	inst_SoundQueue.Music1
	dc.l	zTrack.PlaybackControl
	dc.l	zTrack.VoiceControl
	dc.l	zTrack.TempoDivider
	dc.l	zTrack.DataPointerLow
	dc.l	zTrack.DataPointerHigh
	dc.l	zTrack.Transpose
	dc.l	zTrack.Volume
	dc.l	zTrack.AMSFMSPan
	dc.l	zTrack.VoiceIndex
	dc.l	zTrack.VolFlutter
	dc.l	zTrack.StackPointer
	dc.l	zTrack.DurationTimeout
	dc.l	zTrack.SavedDuration
	dc.l	zTrack.SavedDAC
	dc.l	zTrack.FreqLow
	dc.l	zTrack.FreqHigh
	dc.l	zTrack.NoteFillTimeout
	dc.l	zTrack.NoteFillMaster
	dc.l	zTrack.ModulationPtrLow
	dc.l	zTrack.ModulationPtrHigh
	dc.l	zTrack.ModulationWait
	dc.l	zTrack.ModulationSpeed
	dc.l	zTrack.ModulationDelta
	dc.l	zTrack.ModulationSteps
	dc.l	zTrack.ModulationValLow
	dc.l	zTrack.ModulationValHigh
	dc.l	zTrack.Detune
	dc.l	zTrack.VolTLMask
	dc.l	zTrack.PSGNoise
	dc.l	zTrack.VoicePtrLow
	dc.l	zTrack.VoicePtrHigh
	dc.l	zTrack.TLPtrLow
	dc.l	zTrack.TLPtrHigh
	dc.l	zTrack.LoopCounters
	dc.l	zTrack.GoSubStack
	dc.l	zTrack.len
	dc.l	inst_zTrack.PlaybackControl
	dc.l	inst_zTrack.VoiceControl
	dc.l	inst_zTrack.TempoDivider
	dc.l	inst_zTrack.DataPointerLow
	dc.l	inst_zTrack.DataPointerHigh
	dc.l	inst_zTrack.Transpose
	dc.l	inst_zTrack.Volume
	dc.l	inst_zTrack.AMSFMSPan
	dc.l	inst_zTrack.VoiceIndex
	dc.l	inst_zTrack.VolFlutter
	dc.l	inst_zTrack.StackPointer
	dc.l	inst_zTrack.DurationTimeout
	dc.l	inst_zTrack.SavedDuration
	dc.l	inst_zTrack.SavedDAC
	dc.l	inst_zTrack.FreqLow
	dc.l	inst_zTrack.FreqHigh
	dc.l	inst_zTrack.NoteFillTimeout
	dc.l	inst_zTrack.NoteFillMaster
	dc.l	inst_zTrack.ModulationPtrLow
	dc.l	inst_zTrack.ModulationPtrHigh
	dc.l	inst_zTrack.ModulationWait
	dc.l	inst_zTrack.ModulationSpeed
	dc.l	inst_zTrack.ModulationDelta
	dc.l	inst_zTrack.ModulationSteps
	dc.l	inst_zTrack.ModulationValLow
	dc.l	inst_zTrack.ModulationValHigh
	dc.l	inst_zTrack.Detune
	dc.l	inst_zTrack.VolTLMask
	dc.l	inst_zTrack.PSGNoise
	dc.l	inst_zTrack.VoicePtrLow
	dc.l	inst_zTrack.VoicePtrHigh
	dc.l	inst_zTrack.TLPtrLow
	dc.l	inst_zTrack.TLPtrHigh
	dc.l	inst_zTrack.LoopCounters
	dc.l	inst_zTrack.GoSubStack
	dc.l	zVar.SFXPriorityVal
	dc.l	zVar.TempoTimeout
	dc.l	zVar.CurrentTempo
	dc.l	zVar.StopMusic
	dc.l	zVar.FadeOutCounter
	dc.l	zVar.FadeOutDelay
	dc.l	zVar.Communication
	dc.l	zVar.DACUpdating
	dc.l	zVar.QueueToPlay
	dc.l	zVar.Queue0
	dc.l	zVar.Queue1
	dc.l	zVar.Queue2
	dc.l	zVar.VoiceTblPtr
	dc.l	zVar.FadeInFlag
	dc.l	zVar.FadeInDelay
	dc.l	zVar.FadeInCounter
	dc.l	zVar.TempoMod
	dc.l	zVar.TempoTurbo
	dc.l	zVar.SpeedUpFlag
	dc.l	zVar.DACEnabled
	dc.l	zVar.MusicBankNumber
	dc.l	zVar.IsPalFlag
	dc.l	zVar.len
	dc.l	inst_zVar.SFXPriorityVal
	dc.l	inst_zVar.TempoTimeout
	dc.l	inst_zVar.CurrentTempo
	dc.l	inst_zVar.StopMusic
	dc.l	inst_zVar.FadeOutCounter
	dc.l	inst_zVar.FadeOutDelay
	dc.l	inst_zVar.Communication
	dc.l	inst_zVar.DACUpdating
	dc.l	inst_zVar.QueueToPlay
	dc.l	inst_zVar.Queue0
	dc.l	inst_zVar.Queue1
	dc.l	inst_zVar.Queue2
	dc.l	inst_zVar.VoiceTblPtr
	dc.l	inst_zVar.FadeInFlag
	dc.l	inst_zVar.FadeInDelay
	dc.l	inst_zVar.FadeInCounter
	dc.l	inst_zVar.TempoMod
	dc.l	inst_zVar.TempoTurbo
	dc.l	inst_zVar.SpeedUpFlag
	dc.l	inst_zVar.DACEnabled
	dc.l	inst_zVar.MusicBankNumber
	dc.l	inst_zVar.IsPalFlag
	end
