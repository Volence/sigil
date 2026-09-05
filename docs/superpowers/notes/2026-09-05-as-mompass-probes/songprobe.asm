	CPU 68000
	padding off

z80_ptr function x,(x)<<8&$FF00|(x)>>8&$00FF

FixMusicAndSFXDataBugs = 0
SonicDriverVer = 2
	include "sound/_smps2asm_inc.asm"

	phase $1380
	include "sound/music/93 - Boss.asm"
	dephase

	message "SEEN MOMPASS=\{MOMPASS}"
	dc.b MOMPASS
