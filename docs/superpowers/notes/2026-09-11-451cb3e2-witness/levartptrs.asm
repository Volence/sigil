	cpu 68000
id function ptr,(ptr-PLCPointers)/4
PLCID_Ojz1 =		id(PLCPtr_Ojz1)
PLCID_Ojz2 =		id(PLCPtr_Ojz2)
PalID_OJZ =		0
levartptrs macro plc1,plc2,palette,art,map16x16,map128x128
	dc.l (plc1<<24)|art
	dc.l (plc2<<24)|map16x16
	dc.l (palette<<24)|map128x128
    endm
	org 0
LevelArtPointers:
	levartptrs PLCID_Ojz1, PLCID_Ojz2, PalID_OJZ, Tiles_OJZ, Blocks_OJZ, Chunks_OJZ
Tiles_OJZ:
	dc.w 0
Blocks_OJZ:
	dc.w 0
Chunks_OJZ:
	dc.w 0
Hud:
	jsr LoadLevelLayout
Bad:
	moveq #$1FF,d0
