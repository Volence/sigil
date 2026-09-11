	cpu z80
	org 0
z80_bank_size = 8000h
getZ80BankOffset function label, label # z80_bank_size
getZ80BankBase function label, label - getZ80BankOffset(label)
withinSameZ80Bank function label1, label2, getZ80BankBase(label1) == getZ80BankBase(label2)
MusFlag_SlowerOnPAL = 1 << 6
music_metadata macro DATA,FLAGS
	db	(withinSameZ80Bank(DATA.pointer, MusicPoint2)<<7)|((~~DATA.is_compressed)<<5)|(FLAGS)|(getZ80BankOffset(DATA.pointer)/2)
    endm
Mus_EHZ.pointer = 12344h
Mus_EHZ.is_compressed = 1
Mus_X.pointer = 9876h
Mus_X.is_compressed = 0
MusicPoint2 = 10000h
	music_metadata Mus_EHZ
	music_metadata Mus_X,MusFlag_SlowerOnPAL
	db 0EEh
	end
