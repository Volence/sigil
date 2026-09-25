	cpu 68000
tile_mask = $7FF
RAM_start = $FFFF0000
planeLocH28 function col,line,(line*$50)+(col*2)
make_art_tile function addr,pal,pri,((pri&1)<<15)|((pal&3)<<13)|(addr&tile_mask)
; macro for generating level select strings
levselstr macro str
	save
	codepage	LEVELSELECT
	dc.b strlen(str)-1, str
	restore
    endm

; codepage for level select
	save
	codepage LEVELSELECT
	charset '0','9', 16
	charset 'A','Z', 30
	charset 'a','z', 30
	charset '*', 26
	charset $A9, 27	; '?'
	charset ':', 28
	charset '.', 29
	charset ' ',  0
	restore

		save
		codepage	LEVELSELECT	; This is here so we can use '*' instead of '$1A'

	.blankloop:
		move.w	#make_art_tile(' ',0,0),(a2)+	; Full the remaining space with blank characters

	.stringfull:
		move.w	#make_art_tile('1',0,0),(a2)	; Write (act) '1'
		move.w	#make_art_tile('2',0,0),(a2)	; Write (act) '2'

		; Assuming the last line was the sound test...
		move.w	#make_art_tile(' ',0,0),(a2)	; Get rid of (act) '2'
		move.w	#make_art_tile('*',0,0),(a2)	; Replace that with '*'

		; Overwrite duplicate LAVA REEF 1/2 with 3/4 (obviously, S3 didn't do this)
		move.w	#make_art_tile('3',0,0),(RAM_start+planeLocH28($25,4)).l
		move.w	#make_art_tile('4',0,0),(RAM_start+planeLocH28($25,5)).l

		restore
	dc.b "*AZaz09:. "
LevelSelectText:
		levselstr "ANGEL ISLAND"
		levselstr "HYDROCITY"
		levselstr "MARBLE GARDEN"
		levselstr "CARNIVAL NIGHT"
		levselstr "ICECAP"
		levselstr "LAUNCH BASE"
		levselstr "MUSHROOM HILL"
		levselstr "FLYING BATTERY"
		levselstr "SANDOPOLIS"
		levselstr "LAVA REEF"
		levselstr "LAVA REEF"
		levselstr "SKY SANCTUARY"
		levselstr "DEATHEGG"
		levselstr "THE DOOMSDAY"
		levselstr "BONUS"
		levselstr "SPECIAL STAGE"
		levselstr "SOUND TEST  *"
	dc.b "*AZaz09:. "
